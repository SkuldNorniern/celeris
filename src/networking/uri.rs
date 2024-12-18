use crate::networking::error::NetworkError;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Uri {
    scheme: String,
    host: String,
    port: Option<u16>,
    path: String,
    query: Option<String>,
}

impl Uri {
    pub fn parse(uri: &str) -> Result<Self, NetworkError> {
        // Basic URL parsing - you'll want to expand this
        let parts: Vec<&str> = uri.split("://").collect();
        if parts.len() != 2 {
            return Err(NetworkError::InvalidUri);
        }

        let scheme = parts[0].to_string();
        let remainder = parts[1];

        let (host, path) = remainder.split_once('/').unwrap_or((remainder, ""));

        Ok(Self {
            scheme,
            host: host.to_string(),
            port: None,
            path: format!("/{}", path),
            query: None,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}://{}{}{}",
            self.scheme,
            self.host,
            self.path,
            self.query
                .as_ref()
                .map(|q| format!("?{}", q))
                .unwrap_or_default()
        )
    }
}
