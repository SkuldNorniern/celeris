mod error;
mod http;
mod pool;
mod tcp;
mod uri;
mod user_agent;

pub use error::NetworkError;
pub use uri::Uri;
use pool::ConnectionPool;
use tokio::sync::Mutex;
use std::collections::HashMap;

pub struct NetworkManager {
    cache: Mutex<ResponseCache>,
    cookies: Mutex<CookieJar>,
    pool: ConnectionPool,
}

impl NetworkManager {
    pub fn new() -> Result<Self, NetworkError> {
        Ok(Self {
            cache: Mutex::new(ResponseCache::new()),
            cookies: Mutex::new(CookieJar::new()),
            pool: ConnectionPool::new(),
        })
    }

    pub async fn fetch(&self, url: &str) -> Result<http::Response, NetworkError> {
        if let Some(hit) = self.cache.lock().await.get(url) {
            return Ok(hit);
        }

        let cookie_header = self.cookies.lock().await.get_cookie_header(url);
        
        // Retry logic: retry up to 3 times on failure
        const MAX_RETRIES: usize = 3;
        let mut last_error = None;
        
        for attempt in 0..MAX_RETRIES {
            match self.fetch_with_pool(url, cookie_header.as_deref()).await {
                Ok(response) => {
                    // Check if response indicates a failure that should be retried
                    // (e.g., truncated chunked data, decompression failures)
                    // For now, we'll retry on any error and let fetch_with_pool handle it
                    
                    // Extract Set-Cookie headers and store them
                    self.cookies.lock().await.extract_cookies(url, &response.headers);
                    
                    // Cache successful response
                    self.cache.lock().await.insert(url, &response);
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        // Exponential backoff: wait 100ms, 200ms, 400ms
                        let delay_ms = 100 * (1 << attempt);
                        log::warn!(target: "network", "Request failed (attempt {}/{}), retrying in {}ms: {}", 
                            attempt + 1, MAX_RETRIES, delay_ms, last_error.as_ref().unwrap());
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }
        
        // All retries failed
        Err(last_error.unwrap())
    }

    async fn fetch_with_pool(&self, url: &str, cookie_header: Option<&str>) -> Result<http::Response, NetworkError> {
        const MAX_REDIRECTS: usize = 10;
        const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
        let mut current = url.to_string();

        for _ in 0..MAX_REDIRECTS {
            let uri = Uri::parse(&current)?;
            let mut connection = self.pool.get(&uri).await?;

            let mut builder = http::Request::new()
                .method(http::Method::GET)
                .uri(uri.request_target())
                .header("Host", uri.host())
                .header("Connection", "keep-alive")
                .header("User-Agent", user_agent::user_agent())
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .header("Accept-Encoding", "gzip, deflate, identity")
                .header("Accept-Language", "en-US,en;q=0.9");

            if let Some(cookies) = cookie_header {
                if !cookies.is_empty() {
                    builder = builder.header("Cookie", cookies);
                }
            }

            let request = builder.build()?;
            
            // Wrap send_request with timeout
            let response = tokio::time::timeout(
                REQUEST_TIMEOUT,
                connection.send_request(&request)
            )
            .await
            .map_err(|_| NetworkError::Timeout("Request timed out".to_string()))??;

            // Don't reuse connections for now - causes hangs when the response 
            // reading leaves the connection in a bad state.
            // TODO: Fix response reading to properly drain the connection before reuse.
            drop(connection);

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

// Simple in-memory cookie jar for session persistence
struct CookieJar {
    // Map: domain -> (name -> Cookie)
    cookies: HashMap<String, HashMap<String, Cookie>>,
}

#[derive(Clone)]
struct Cookie {
    name: String,
    value: String,
    path: String,
    // secure: bool, // For future: only send over HTTPS
    // http_only: bool, // For future: not accessible via JS
}

impl CookieJar {
    fn new() -> Self {
        Self {
            cookies: HashMap::new(),
        }
    }

    // Extract cookies from Set-Cookie headers and store them
    fn extract_cookies(&mut self, url: &str, headers: &http::Headers) {
        let domain = match Uri::parse(url) {
            Ok(uri) => uri.host().to_lowercase(),
            Err(_) => return,
        };

        // Process all Set-Cookie headers (there can be multiple)
        if let Some(set_cookies) = headers.get_all("set-cookie") {
            for set_cookie in set_cookies {
                if let Some(cookie) = parse_set_cookie(set_cookie, &domain) {
                    self.cookies
                        .entry(domain.clone())
                        .or_default()
                        .insert(cookie.name.clone(), cookie);
                }
            }
        }
    }

    // Build Cookie header for a request
    fn get_cookie_header(&self, url: &str) -> Option<String> {
        let uri = Uri::parse(url).ok()?;
        let domain = uri.host().to_lowercase();
        let path = uri.path();

        let domain_cookies = self.cookies.get(&domain)?;
        if domain_cookies.is_empty() {
            return None;
        }

        let cookies: Vec<String> = domain_cookies
            .values()
            .filter(|c| path.starts_with(&c.path))
            .map(|c| format!("{}={}", c.name, c.value))
            .collect();

        if cookies.is_empty() {
            None
        } else {
            Some(cookies.join("; "))
        }
    }
}

fn parse_set_cookie(header_value: &str, _default_domain: &str) -> Option<Cookie> {
    // Format: name=value; Path=/; Domain=...; Secure; HttpOnly
    let mut parts = header_value.split(';');
    let name_value = parts.next()?.trim();
    let (name, value) = name_value.split_once('=')?;

    let mut path = "/".to_string();

    for attr in parts {
        let attr = attr.trim();
        if let Some((key, val)) = attr.split_once('=') {
            let key_lower = key.trim().to_lowercase();
            if key_lower == "path" {
                path = val.trim().to_string();
            }
            // We ignore Domain, Secure, HttpOnly, etc. for simplicity
        }
    }

    Some(Cookie {
        name: name.trim().to_string(),
        value: value.trim().to_string(),
        path,
    })
}
