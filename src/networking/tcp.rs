use crate::networking::{error::NetworkError, http, uri::Uri};
use flate2::read::{GzDecoder, DeflateDecoder};
use rustls::pki_types::ServerName;
use std::io::Read;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

pub enum Connection {
    Plain(TcpStream),
    Tls(tokio_rustls::client::TlsStream<TcpStream>),
}

pub struct TcpConnection {
    connection: Connection,
    host: String,
    keep_alive: bool,
}

impl TcpConnection {
    const MAX_DECODED_BODY_BYTES: usize = 32 * 1024 * 1024; // 32 MiB safety cap

    pub async fn connect(uri: &Uri) -> Result<Self, NetworkError> {
        let is_https = uri.scheme() == "https";
        let default_port = if is_https { 443 } else { 80 };
        let port = uri.port().unwrap_or(default_port);
        let addr = format!("{}:{}", uri.host(), port);

        let tcp_stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        let connection = if is_https {
            // Setup TLS
            let mut root_store = RootCertStore::empty();

            // Add root certificates
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            let config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

            let connector = TlsConnector::from(Arc::new(config));

            // Clone the host string to satisfy the 'static lifetime requirement
            let server_name = ServerName::try_from(uri.host().to_string())
                .map_err(|e| NetworkError::TlsError(e.to_string()))?;

            let tls_stream = connector
                .connect(server_name, tcp_stream)
                .await
                .map_err(|e| NetworkError::TlsError(e.to_string()))?;

            Connection::Tls(tls_stream)
        } else {
            Connection::Plain(tcp_stream)
        };

        Ok(Self {
            connection,
            host: uri.host().to_string(),
            keep_alive: true,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns true if the connection can be reused for another request.
    pub fn is_keep_alive(&self) -> bool {
        self.keep_alive
    }

    pub async fn send_request(
        &mut self,
        request: &http::Request,
    ) -> Result<http::Response, NetworkError> {
        // Send request
        match &mut self.connection {
            Connection::Plain(stream) => {
                stream
                    .write_all(&request.to_bytes())
                    .await
                    .map_err(|e| NetworkError::SendFailed(e.to_string()))?;
            }
            Connection::Tls(stream) => {
                stream
                    .write_all(&request.to_bytes())
                    .await
                    .map_err(|e| NetworkError::SendFailed(e.to_string()))?;
            }
        }

        // Read response with keep-alive support: don't wait for EOF,
        // instead read headers first, then read exact body length.
        let response_data = self.read_response().await?;

        if response_data.is_empty() {
            return Err(NetworkError::ReceiveFailed(
                "Empty response received".to_string(),
            ));
        }

        self.parse_response(response_data)
    }

    /// Read an HTTP response, handling both keep-alive and close connections.
    async fn read_response(&mut self) -> Result<Vec<u8>, NetworkError> {
        let mut data = Vec::new();
        let mut buffer = [0u8; 8192];

        // First, read until we have the full headers
        let header_end = loop {
            let n = self.read_some(&mut buffer).await?;
            if n == 0 {
                // Connection closed before headers complete
                break find_header_end(&data).unwrap_or(data.len());
            }
            data.extend_from_slice(&buffer[..n]);
            if let Some(end) = find_header_end(&data) {
                break end;
            }
        };

        // Parse headers to determine body length strategy
        let header_str = String::from_utf8_lossy(&data[..header_end]);
        let mut content_length: Option<usize> = None;
        let mut is_chunked = false;
        let mut connection_close = false;

        for line in header_str.split("\r\n").skip(1) {
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                let name_lower = name.trim().to_lowercase();
                let value_trim = value.trim();
                match name_lower.as_str() {
                    "content-length" => {
                        content_length = value_trim.parse().ok();
                    }
                    "transfer-encoding" => {
                        is_chunked = value_trim
                            .split(',')
                            .any(|v| v.trim().eq_ignore_ascii_case("chunked"));
                    }
                    "connection" => {
                        connection_close = value_trim.eq_ignore_ascii_case("close");
                    }
                    _ => {}
                }
            }
        }

        // Update keep-alive status
        self.keep_alive = !connection_close;

        // Now read the body
        let body_start = header_end;

        if is_chunked {
            // For chunked, we need to read until we see the terminating chunk (0\r\n\r\n)
            // The terminator can appear anywhere after the body start, followed by optional trailers
            while !has_chunked_terminator(&data[body_start..]) {
                let n = self.read_some(&mut buffer).await?;
                if n == 0 {
                    log::debug!(target: "network", "EOF while reading chunked body");
                    break;
                }
                data.extend_from_slice(&buffer[..n]);
                // Safety check for very large responses
                if data.len() > Self::MAX_DECODED_BODY_BYTES + 1024 * 1024 {
                    log::warn!(target: "network", "Chunked body exceeds max size, truncating");
                    break;
                }
            }
        } else if let Some(len) = content_length {
            // Read exactly len bytes for the body
            let target = body_start + len;
            while data.len() < target {
                let n = self.read_some(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                data.extend_from_slice(&buffer[..n]);
            }
        } else if connection_close {
            // No Content-Length and not chunked, but Connection: close - read until EOF
            loop {
                let n = self.read_some(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                data.extend_from_slice(&buffer[..n]);
            }
            self.keep_alive = false;
        } else {
            // No Content-Length, not chunked, and keep-alive - this is malformed.
            // For HTTP/1.1 keep-alive, server MUST send Content-Length or chunked.
            // Assume zero-length body and mark connection as non-reusable.
            log::warn!(target: "network", "Keep-alive response missing Content-Length/chunked, assuming empty body");
            self.keep_alive = false;
        }

        Ok(data)
    }

    /// Read from the underlying stream with timeout, returning bytes read or 0 on EOF.
    async fn read_some(&mut self, buffer: &mut [u8]) -> Result<usize, NetworkError> {
        const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
        
        let read_future = async {
            match &mut self.connection {
                Connection::Plain(stream) => stream
                    .read(buffer)
                    .await
                    .map_err(|e| NetworkError::ReceiveFailed(e.to_string())),
                Connection::Tls(stream) => match stream.read(buffer).await {
                    Ok(n) => Ok(n),
                    Err(e) => {
                        // TLS close_notify is expected EOF
                        if e.to_string().contains("close_notify") {
                            Ok(0)
                        } else {
                            Err(NetworkError::ReceiveFailed(e.to_string()))
                        }
                    }
                },
            }
        };

        tokio::time::timeout(READ_TIMEOUT, read_future)
            .await
            .map_err(|_| NetworkError::Timeout("Read timed out".to_string()))?
    }

    fn parse_response(&self, data: Vec<u8>) -> Result<http::Response, NetworkError> {
        let header_end = find_header_end(&data).ok_or_else(|| {
            NetworkError::ParseError("Missing header terminator (\\r\\n\\r\\n)".to_string())
        })?;

        // Parse status line + headers from the header section only.
        let header_bytes = &data[..header_end];
        let header_str = String::from_utf8_lossy(header_bytes);
        let mut lines = header_str.split("\r\n");

        let status_line = lines
            .next()
            .ok_or_else(|| NetworkError::ParseError("Empty response".to_string()))?;

        let mut status_parts = status_line.split_whitespace();
        let version_str = status_parts
            .next()
            .ok_or_else(|| NetworkError::ParseError("Missing HTTP version".to_string()))?;
        let version = match version_str {
            "HTTP/1.1" => http::Version::Http11,
            "HTTP/1.0" => http::Version::Http10,
            _ => return Err(NetworkError::ParseError("Invalid HTTP version".to_string())),
        };

        let code = status_parts
            .next()
            .ok_or_else(|| NetworkError::ParseError("Missing status code".to_string()))?
            .parse::<u16>()
            .map_err(|_| NetworkError::ParseError("Invalid status code".to_string()))?;

        let status_text = status_parts.collect::<Vec<_>>().join(" ");
        if status_text.is_empty() {
            return Err(NetworkError::ParseError("Missing status text".to_string()));
        }

        let mut headers = http::Headers::new();
        for line in lines {
            if line.is_empty() {
                break;
            }
            let (name, value) = line.split_once(':').ok_or_else(|| {
                NetworkError::HeaderParseError(format!("Invalid header line: {line}"))
            })?;
            headers.append(name.trim().to_string(), value.trim().to_string());
        }

        let mut body = data[header_end..].to_vec();

        // Decode Transfer-Encoding: chunked if present. Many sites (including https://nornity.com)
        // use chunked responses, and the chunk-size lines must not leak into HTML parsing.
        if is_transfer_encoding_chunked(&headers) {
            body = decode_chunked_body(&body, Self::MAX_DECODED_BODY_BYTES)?;
        } else if let Some(content_length) = headers.get("content-length") {
            if let Ok(len) = content_length.trim().parse::<usize>() {
                if body.len() >= len {
                    body.truncate(len);
                }
            }
        }

        // Decompress Content-Encoding: gzip or deflate
        body = decompress_body(&headers, body)?;

        Ok(http::Response {
            version,
            status: http::Status {
                code,
                text: status_text,
            },
            headers,
            body,
        })
    }
}

fn decompress_body(headers: &http::Headers, body: Vec<u8>) -> Result<Vec<u8>, NetworkError> {
    let Some(encoding) = headers.get("content-encoding") else {
        return Ok(body);
    };

    // If body is empty, nothing to decompress
    if body.is_empty() {
        log::debug!(target: "network", "Body is empty, skipping decompression");
        return Ok(body);
    }

    let encoding = encoding.trim().to_lowercase();
    match encoding.as_str() {
        "gzip" | "x-gzip" => {
            // Check if body looks like gzip (starts with gzip magic bytes: 1f 8b)
            if body.len() < 2 || body[0] != 0x1f || body[1] != 0x8b {
                log::warn!(target: "network", "Content-Encoding says gzip but body doesn't have gzip magic bytes, returning as-is");
                return Ok(body);
            }
            
            let mut decoder = GzDecoder::new(&body[..]);
            let mut decompressed = Vec::new();
            match decoder.read_to_end(&mut decompressed) {
                Ok(_) => {
                    log::debug!(target: "network", "Successfully decompressed gzip body: {} -> {} bytes", body.len(), decompressed.len());
                    Ok(decompressed)
                }
                Err(e) => {
                    log::warn!(target: "network", "Gzip decompression failed: {}, body len: {}, returning body as-is", e, body.len());
                    // Fallback: return body as-is in case it's already decompressed
                    // Some servers incorrectly set Content-Encoding: gzip when body is already plain
                    Ok(body)
                }
            }
        }
        "deflate" => {
            let mut decoder = DeflateDecoder::new(&body[..]);
            let mut decompressed = Vec::new();
            match decoder.read_to_end(&mut decompressed) {
                Ok(_) => {
                    log::debug!(target: "network", "Successfully decompressed deflate body: {} -> {} bytes", body.len(), decompressed.len());
                    Ok(decompressed)
                }
                Err(e) => {
                    log::warn!(target: "network", "Deflate decompression failed: {}, body len: {}, returning body as-is", e, body.len());
                    // Fallback: return body as-is
                    Ok(body)
                }
            }
        }
        "identity" | "" => Ok(body),
        other => {
            // Unknown encoding, return body as-is and log warning
            log::warn!(target: "network", "Unknown Content-Encoding: {}, returning raw body", other);
            Ok(body)
        }
    }
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    for (i, window) in data.windows(4).enumerate() {
        if window == b"\r\n\r\n" {
            return Some(i + 4);
        }
    }
    None
}

/// Check if chunked body contains the terminating chunk (0\r\n followed by trailers and \r\n)
fn has_chunked_terminator(body: &[u8]) -> bool {
    // Look for \r\n0\r\n which indicates start of terminating chunk
    // The full terminator is: \r\n0\r\n(<trailers>)?\r\n
    // We look for the simpler pattern of just ending with 0\r\n\r\n or having \r\n0\r\n\r\n
    if body.is_empty() {
        return false;
    }
    
    // Check for terminator at end
    if body.ends_with(b"0\r\n\r\n") || body.ends_with(b"\r\n0\r\n\r\n") {
        return true;
    }
    
    // Also look for the terminating chunk pattern within the data
    // A chunked terminator is: CRLF "0" CRLF (optional-trailers) CRLF
    // The key signature is CRLF "0" CRLF CRLF (no trailers) or CRLF "0" CRLF <header> CRLF CRLF
    for i in 0..body.len().saturating_sub(4) {
        // Look for \r\n0\r\n
        if body.get(i..i+5) == Some(b"\r\n0\r\n") {
            // Check if this is followed by another CRLF (end of trailers)
            let trailer_start = i + 5;
            let mut j = trailer_start;
            // Skip any trailer lines
            while j + 1 < body.len() {
                if body[j] == b'\r' && body[j + 1] == b'\n' {
                    // Found CRLF - either end of trailer line or end of trailers
                    if j == trailer_start || (j > trailer_start && body.get(j-1) == Some(&b'\n')) {
                        // Empty line = end of trailers
                        return true;
                    }
                    // Look for the next CRLF to see if it's the end
                    let next = j + 2;
                    if next + 1 < body.len() && body[next] == b'\r' && body[next + 1] == b'\n' {
                        return true;
                    }
                }
                j += 1;
            }
            // If we're at the very end, assume terminator
            if j >= body.len().saturating_sub(2) {
                return true;
            }
        }
    }
    
    false
}

fn is_transfer_encoding_chunked(headers: &http::Headers) -> bool {
    let Some(te) = headers.get("transfer-encoding") else {
        return false;
    };
    te.split(',')
        .map(|v| v.trim())
        .any(|v| v.eq_ignore_ascii_case("chunked"))
}

fn decode_chunked_body(input: &[u8], max_decoded_size: usize) -> Result<Vec<u8>, NetworkError> {
    // Handle empty input gracefully
    if input.is_empty() {
        log::debug!(target: "network", "Chunked body is empty");
        return Ok(Vec::new());
    }
    
    let mut out = Vec::new();
    let mut i = 0usize;

    loop {
        // Skip any leading whitespace/CRLF (some servers add extra)
        while i < input.len() && (input[i] == b'\r' || input[i] == b'\n' || input[i] == b' ') {
            i += 1;
        }
        
        if i >= input.len() {
            // End of input reached
            break;
        }
        
        let line_end = match find_crlf(input, i) {
            Some(end) => end,
            None => {
                // No CRLF found - might be truncated data or end of stream
                // Try to parse what we have as a chunk size anyway
                log::debug!(target: "network", "Chunked: no CRLF found at position {}, input len {}", i, input.len());
                // If we already have data, return it; otherwise error
                if !out.is_empty() {
                    log::warn!(target: "network", "Chunked encoding truncated, returning partial data");
                    return Ok(out);
                }
                return Err(NetworkError::ParseError(
                    "Invalid chunked encoding: missing CRLF after size".to_string()
                ));
            }
        };
        
        let size_line = &input[i..line_end];
        i = line_end + 2;

        // Allow chunk extensions: "<hex>;ext=..."
        let size_field = size_line
            .split(|b| *b == b';')
            .next()
            .unwrap_or(size_line);
        let size_str = String::from_utf8_lossy(size_field);
        let trimmed = size_str.trim();
        
        // Handle empty size field
        if trimmed.is_empty() {
            continue;
        }
        
        let size = match usize::from_str_radix(trimmed, 16) {
            Ok(s) => s,
            Err(_) => {
                log::debug!(target: "network", "Invalid chunk size '{}', stopping", trimmed);
                break;
            }
        };

        if size == 0 {
            // Trailers: 0\r\n(<header>\r\n)*\r\n
            loop {
                match find_crlf(input, i) {
                    Some(trailer_end) if trailer_end == i => break,
                    Some(trailer_end) => i = trailer_end + 2,
                    None => break, // No more trailers, done
                }
            }
            break;
        }

        if out.len().saturating_add(size) > max_decoded_size {
            return Err(NetworkError::TooLargeResponse);
        }

        let chunk_end = match i.checked_add(size) {
            Some(end) => end,
            None => {
                log::warn!(target: "network", "Chunk size overflow, returning partial data");
                break;
            }
        };
        
        if chunk_end > input.len() {
            // Truncated chunk - take what we can
            log::warn!(target: "network", "Chunked data truncated (expected {} bytes, have {})", size, input.len() - i);
            if i < input.len() {
                out.extend_from_slice(&input[i..]);
            }
            break;
        }

        out.extend_from_slice(&input[i..chunk_end]);
        i = chunk_end;

        // Each chunk is followed by CRLF.
        if input.get(i..i + 2) == Some(b"\r\n") {
            i += 2;
        } else if i < input.len() && input[i] == b'\n' {
            // Some servers use just LF
            i += 1;
        }
        // If no CRLF, just continue - might be end of data
    }

    Ok(out)
}

fn find_crlf(buf: &[u8], start: usize) -> Option<usize> {
    let mut idx = start;
    while idx + 1 < buf.len() {
        if buf[idx] == b'\r' && buf[idx + 1] == b'\n' {
            return Some(idx);
        }
        idx += 1;
    }
    None
}
