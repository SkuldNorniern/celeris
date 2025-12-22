mod error;
mod http;
mod tcp;
mod uri;

pub use error::NetworkError;
pub use uri::Uri;
use tcp::TcpConnection;
use tokio::sync::Mutex;
use std::collections::HashMap;

pub struct NetworkManager {
    client: HttpClient,
    cache: Mutex<ResponseCache>,
}

pub struct HttpClient {
    timeout: std::time::Duration,
}

impl NetworkManager {
    pub fn new() -> Result<Self, NetworkError> {
        Ok(Self {
            client: HttpClient::new(),
            cache: Mutex::new(ResponseCache::new()),
        })
    }

    pub async fn fetch(&self, url: &str) -> Result<http::Response, NetworkError> {
        if let Some(hit) = self.cache.lock().await.get(url) {
            return Ok(hit);
        }

        let response = self.client.get(url).await?;
        self.cache.lock().await.insert(url, &response);
        Ok(response)
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(30),
        }
    }

    pub async fn get(&self, url: &str) -> Result<http::Response, NetworkError> {
        const MAX_REDIRECTS: usize = 10;
        let mut current = url.to_string();

        for _ in 0..MAX_REDIRECTS {
            let uri = Uri::parse(&current)?;
            let mut connection = TcpConnection::connect(&uri).await?;

            let request = http::Request::new()
                .method(http::Method::GET)
                .uri(uri.request_target())
                .header("Host", uri.host())
                .header("Connection", "close")
                .header("User-Agent", "Celeris/0.1")
                .header("Accept", "*/*")
                // Prefer uncompressed responses for now. This avoids needing gzip/br support.
                .header("Accept-Encoding", "identity")
                .build()?;

            let response = connection.send_request(&request).await?;
            if is_redirect_status(response.status.code) {
                if let Some(location) = response.headers.get("location") {
                    current = uri.resolve_reference(location)?;
                    continue;
                }
            }
            return Ok(response);
        }

        Err(NetworkError::TooManyRedirects)
    }
}

fn is_redirect_status(code: u16) -> bool {
    matches!(code, 301 | 302 | 303 | 307 | 308)
}

struct ResponseCache {
    entries: HashMap<String, http::Response>,
    current_body_bytes: usize,
    max_body_bytes: usize,
    max_entry_body_bytes: usize,
}

impl ResponseCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            current_body_bytes: 0,
            max_body_bytes: 16 * 1024 * 1024, // 16 MiB
            max_entry_body_bytes: 2 * 1024 * 1024, // 2 MiB
        }
    }

    fn get(&self, url: &str) -> Option<http::Response> {
        self.entries.get(url).cloned()
    }

    fn insert(&mut self, url: &str, response: &http::Response) {
        // Keep cache simple and safe: only cache reasonably sized bodies.
        if response.body.len() > self.max_entry_body_bytes {
            return;
        }

        // If we'd exceed the total budget, clear the cache (no LRU yet).
        if self.current_body_bytes.saturating_add(response.body.len()) > self.max_body_bytes {
            self.entries.clear();
            self.current_body_bytes = 0;
        }

        // Replacing an existing entry: subtract old size first.
        if let Some(old) = self.entries.get(url) {
            self.current_body_bytes = self.current_body_bytes.saturating_sub(old.body.len());
        }

        self.entries.insert(url.to_string(), response.clone());
        self.current_body_bytes = self.current_body_bytes.saturating_add(response.body.len());
    }
}
