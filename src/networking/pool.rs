use crate::networking::{error::NetworkError, tcp::TcpConnection, uri::Uri};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

/// Default timeout for connection and request operations
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Simple connection pool for HTTP keep-alive connections.
/// Keyed by host:port, stores idle connections with a TTL.
pub struct ConnectionPool {
    connections: Mutex<HashMap<String, PooledConnection>>,
    max_idle_time: Duration,
    connect_timeout: Duration,
}

struct PooledConnection {
    connection: TcpConnection,
    last_used: Instant,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
            max_idle_time: Duration::from_secs(30),
            connect_timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Get a connection for the given URI, either from the pool or by creating a new one.
    pub async fn get(&self, uri: &Uri) -> Result<TcpConnection, NetworkError> {
        let key = pool_key(uri);
        
        // Try to get an existing connection from the pool
        let mut pool = self.connections.lock().await;
        if let Some(pooled) = pool.remove(&key) {
            if pooled.last_used.elapsed() < self.max_idle_time {
                log::debug!(target: "network", "Reusing pooled connection for {}", key);
                return Ok(pooled.connection);
            }
            log::debug!(target: "network", "Dropping expired connection for {}", key);
        }
        drop(pool);

        // Create a new connection with timeout
        log::debug!(target: "network", "Creating new connection for {}", key);
        tokio::time::timeout(self.connect_timeout, TcpConnection::connect(uri))
            .await
            .map_err(|_| NetworkError::Timeout("Connection timed out".to_string()))?
    }

    /// Return a connection to the pool for reuse.
    /// The connection should still be valid (not closed by the server).
    pub async fn put(&self, uri: &Uri, connection: TcpConnection) {
        let key = pool_key(uri);
        let mut pool = self.connections.lock().await;
        
        // Evict old connections if pool is getting large
        if pool.len() >= 16 {
            let now = Instant::now();
            pool.retain(|_, v| now.duration_since(v.last_used) < self.max_idle_time);
        }

        pool.insert(key, PooledConnection {
            connection,
            last_used: Instant::now(),
        });
    }

    /// Evict expired connections from the pool.
    #[allow(dead_code)]
    pub async fn evict_expired(&self) {
        let mut pool = self.connections.lock().await;
        let now = Instant::now();
        pool.retain(|_, v| now.duration_since(v.last_used) < self.max_idle_time);
    }
}

fn pool_key(uri: &Uri) -> String {
    let port = uri.port().unwrap_or(if uri.scheme() == "https" { 443 } else { 80 });
    format!("{}:{}:{}", uri.scheme(), uri.host(), port)
}

