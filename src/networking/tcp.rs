use crate::networking::{error::NetworkError, http, uri::Uri};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct TcpConnection {
    stream: TcpStream,
    host: String,
}

impl TcpConnection {
    pub async fn connect(uri: &Uri) -> Result<Self, NetworkError> {
        let port = uri.port().unwrap_or(80);
        let addr = format!("{}:{}", uri.host(), port);

        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            stream,
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
        self.stream
            .write_all(&request.to_bytes())
            .await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

        // Read response
        let mut response_data = Vec::new();
        self.stream
            .read_to_end(&mut response_data)
            .await
            .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

        self.parse_response(response_data)
    }

    fn parse_response(&self, data: Vec<u8>) -> Result<http::Response, NetworkError> {
        let headers = http::Headers::new();

        Ok(http::Response {
            version: http::Version::Http11,
            status: http::Status {
                code: 200,
                text: "OK".to_string(),
            },
            headers,
            body: data,
        })
    }
}
