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
        let parts: Vec<&str> = uri.split("://").collect();
        if parts.len() != 2 {
            return Err(NetworkError::InvalidUri);
        }

        let scheme = parts[0].to_string();
        if scheme != "http" && scheme != "https" {
            return Err(NetworkError::InvalidUri);
        }

        let remainder = parts[1];
        let (authority, path) = remainder.split_once('/').unwrap_or((remainder, ""));
        
        // Handle port in authority
        let (host, port) = if let Some((h, p)) = authority.split_once(':') {
            (h.to_string(), Some(p.parse().map_err(|_| NetworkError::InvalidUri)?))
        } else {
            (authority.to_string(), None)
        };

        Ok(Self {
            scheme,
            host,
            port,
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

    pub fn scheme(&self) -> &str {
        &self.scheme
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
