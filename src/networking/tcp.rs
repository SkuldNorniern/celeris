use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use rustls::pki_types::ServerName;
use std::sync::Arc;
use crate::networking::{http, error::NetworkError, uri::Uri};

pub enum Connection {
    Plain(TcpStream),
    Tls(tokio_rustls::client::TlsStream<TcpStream>),
}

pub struct TcpConnection {
    connection: Connection,
    host: String,
}

impl TcpConnection {
    pub async fn connect(uri: &Uri) -> Result<Self, NetworkError> {
        let is_https = uri.scheme() == "https";
        let default_port = if is_https { 443 } else { 80 };
        let port = uri.port().unwrap_or(default_port);
        let addr = format!("{}:{}", uri.host(), port);
        
        let tcp_stream = TcpStream::connect(&addr).await
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

            let tls_stream = connector.connect(server_name, tcp_stream).await
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

    pub async fn send_request(&mut self, request: &http::Request) -> Result<http::Response, NetworkError> {
        // Send request
        match &mut self.connection {
            Connection::Plain(stream) => {
                stream.write_all(&request.to_bytes()).await
                    .map_err(|e| NetworkError::SendFailed(e.to_string()))?;
            }
            Connection::Tls(stream) => {
                stream.write_all(&request.to_bytes()).await
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
            return Err(NetworkError::ReceiveFailed("Empty response received".to_string()));
        }

        self.parse_response(response_data)
    }

    fn parse_response(&self, data: Vec<u8>) -> Result<http::Response, NetworkError> {
        // Convert data to string for parsing the status line and headers
        let response_str = String::from_utf8_lossy(&data);
        let mut lines = response_str.lines();
        
        // Parse status line
        let status_line = lines.next()
            .ok_or_else(|| NetworkError::ParseError("Empty response".to_string()))?;
        
        let mut status_parts = status_line.split_whitespace();
        
        // Parse HTTP version
        let version = status_parts.next()
            .ok_or_else(|| NetworkError::ParseError("Missing HTTP version".to_string()))?;
        let version = match version {
            "HTTP/1.1" => http::Version::Http11,
            "HTTP/1.0" => http::Version::Http10,
            _ => return Err(NetworkError::ParseError("Invalid HTTP version".to_string()))
        };
        
        // Parse status code
        let code = status_parts.next()
            .ok_or_else(|| NetworkError::ParseError("Missing status code".to_string()))?
            .parse::<u16>()
            .map_err(|_| NetworkError::ParseError("Invalid status code".to_string()))?;
        
        // Parse status text
        let text = status_parts.collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            return Err(NetworkError::ParseError("Missing status text".to_string()));
        }

        let headers = http::Headers::new(); // TODO: Parse headers in next iteration
        
        // Find the separation between headers and body (empty line)
        let mut body_start = 0;
        for (i, window) in data.windows(4).enumerate() {
            if window == b"\r\n\r\n" {
                body_start = i + 4;
                break;
            }
        }
        
        Ok(http::Response {
            version,
            status: http::Status {
                code,
                text: text.to_string(),
            },
            headers,
            body: data[body_start..].to_vec(),
        })
    }
} 