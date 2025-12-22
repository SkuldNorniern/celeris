use crate::networking::{error::NetworkError, http, uri::Uri};
use rustls::pki_types::ServerName;
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
        })
    }

    pub fn host(&self) -> &str {
        &self.host
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

        // Read response
        let mut response_data = Vec::new();
        let mut buffer = [0; 8192]; // Use a fixed-size buffer for reading

        match &mut self.connection {
            Connection::Plain(stream) => {
                loop {
                    match stream.read(&mut buffer).await {
                        Ok(0) => break, // Connection closed
                        Ok(n) => response_data.extend_from_slice(&buffer[..n]),
                        Err(e) => return Err(NetworkError::ReceiveFailed(e.to_string())),
                    }
                }
            }
            Connection::Tls(stream) => {
                loop {
                    match stream.read(&mut buffer).await {
                        Ok(0) => break, // Connection closed
                        Ok(n) => response_data.extend_from_slice(&buffer[..n]),
                        Err(e) => {
                            // Ignore EOF errors that mention close_notify
                            if e.to_string().contains("close_notify") {
                                break;
                            }
                            return Err(NetworkError::ReceiveFailed(e.to_string()));
                        }
                    }
                }
            }
        }

        if response_data.is_empty() {
            return Err(NetworkError::ReceiveFailed(
                "Empty response received".to_string(),
            ));
        }

        self.parse_response(response_data)
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
            headers.insert(name.trim().to_string(), value.trim().to_string());
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

fn find_header_end(data: &[u8]) -> Option<usize> {
    for (i, window) in data.windows(4).enumerate() {
        if window == b"\r\n\r\n" {
            return Some(i + 4);
        }
    }
    None
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
    let mut out = Vec::new();
    let mut i = 0usize;

    loop {
        let line_end = find_crlf(input, i).ok_or_else(|| {
            NetworkError::ParseError("Invalid chunked encoding: missing CRLF after size".to_string())
        })?;
        let size_line = &input[i..line_end];
        i = line_end + 2;

        // Allow chunk extensions: "<hex>;ext=..."
        let size_field = size_line
            .split(|b| *b == b';')
            .next()
            .unwrap_or(size_line);
        let size_str = String::from_utf8_lossy(size_field);
        let size = usize::from_str_radix(size_str.trim(), 16).map_err(|_| {
            NetworkError::ParseError(format!(
                "Invalid chunk size field in chunked encoding: '{}'",
                size_str.trim()
            ))
        })?;

        if size == 0 {
            // Trailers: 0\r\n(<header>\r\n)*\r\n
            loop {
                let trailer_end = find_crlf(input, i).ok_or_else(|| {
                    NetworkError::ParseError(
                        "Invalid chunked encoding: missing CRLF in trailers".to_string(),
                    )
                })?;
                if trailer_end == i {
                    break;
                }
                i = trailer_end + 2;
            }
            break;
        }

        if out.len().saturating_add(size) > max_decoded_size {
            return Err(NetworkError::TooLargeResponse);
        }

        let chunk_end = i.checked_add(size).ok_or_else(|| {
            NetworkError::ParseError("Invalid chunked encoding: chunk size overflow".to_string())
        })?;
        if chunk_end > input.len() {
            return Err(NetworkError::ParseError(
                "Invalid chunked encoding: chunk data beyond buffer".to_string(),
            ));
        }

        out.extend_from_slice(&input[i..chunk_end]);
        i = chunk_end;

        // Each chunk is followed by CRLF.
        if input.get(i..i + 2) != Some(b"\r\n") {
            return Err(NetworkError::ParseError(
                "Invalid chunked encoding: missing CRLF after chunk data".to_string(),
            ));
        }
        i += 2;
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
