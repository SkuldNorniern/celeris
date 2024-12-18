mod error;
mod http;
mod tcp;
mod uri;

pub use error::NetworkError;
use tcp::TcpConnection;
use uri::Uri;

pub struct NetworkManager {
    client: HttpClient,
}

pub struct HttpClient {
    timeout: std::time::Duration,
}

impl NetworkManager {
    pub fn new() -> Result<Self, NetworkError> {
        Ok(Self {
            client: HttpClient::new(),
        })
    }

    pub async fn fetch(&self, url: &str) -> Result<http::Response, NetworkError> {
        self.client.get(url).await
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(30),
        }
    }

    pub async fn get(&self, url: &str) -> Result<http::Response, NetworkError> {
        let uri = Uri::parse(url)?;
        let mut connection = TcpConnection::connect(&uri).await?;

        let request = http::Request::new()
            .method(http::Method::GET)
            .uri(uri.to_string())
            .header("Host", connection.host())
            .header("Connection", "close")
            .build()?;

        connection.send_request(&request).await
    }
}
