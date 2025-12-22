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
        let (scheme_part, remainder) = uri.split_once("://").ok_or(NetworkError::InvalidUri)?;
        let scheme = scheme_part.to_string();
        if scheme != "http" && scheme != "https" {
            return Err(NetworkError::InvalidUri);
        }

        let (authority, path_and_more) = remainder.split_once('/').unwrap_or((remainder, ""));

        // Handle port in authority
        let (host, port) = if let Some((h, p)) = authority.split_once(':') {
            (
                h.to_string(),
                Some(p.parse().map_err(|_| NetworkError::InvalidUri)?),
            )
        } else {
            (authority.to_string(), None)
        };

        let (path_and_query, _) = path_and_more.split_once('#').unwrap_or((path_and_more, ""));
        let (path_part, query) = path_and_query.split_once('?').unwrap_or((path_and_query, ""));
        let path = if path_part.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", path_part)
        };
        let query = if query.is_empty() { None } else { Some(query.to_string()) };

        Ok(Self {
            scheme,
            host,
            port,
            path,
            query,
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

    pub fn request_target(&self) -> String {
        if let Some(q) = &self.query {
            let mut out = String::with_capacity(self.path.len() + 1 + q.len());
            out.push_str(&self.path);
            out.push('?');
            out.push_str(q);
            out
        } else {
            self.path.clone()
        }
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn origin(&self) -> String {
        match self.port {
            Some(port) => format!("{}://{}:{}", self.scheme, self.host, port),
            None => format!("{}://{}", self.scheme, self.host),
        }
    }

    pub fn resolve_reference(&self, reference: &str) -> Result<String, NetworkError> {
        let reference = reference.trim();
        if reference.is_empty() {
            return Ok(self.to_string());
        }

        if reference.starts_with("http://") || reference.starts_with("https://") {
            return Ok(reference.to_string());
        }

        // Scheme-relative URL: //cdn.example.com/...
        if let Some(rest) = reference.strip_prefix("//") {
            return Ok(format!("{}://{}", self.scheme, rest));
        }

        // Fragment-only reference: keep current URL.
        if reference.starts_with('#') {
            return Ok(self.to_string());
        }

        // Query-only reference: keep path, replace query.
        if let Some(q) = reference.strip_prefix('?') {
            let mut out = self.origin();
            out.push_str(&self.path);
            if !q.is_empty() {
                out.push('?');
                out.push_str(q);
            }
            return Ok(out);
        }

        // Absolute-path reference.
        if reference.starts_with('/') {
            let mut out = self.origin();
            out.push_str(reference);
            return Ok(out);
        }

        // Relative-path reference.
        let base_dir = base_dir_of_path(&self.path);
        let combined = format!("{}{}", base_dir, reference);
        let normalized = normalize_path(&combined);

        let mut out = self.origin();
        out.push_str(&normalized);
        Ok(out)
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

fn base_dir_of_path(path: &str) -> &str {
    // Always returns a string ending with '/', so we can safely append a relative reference.
    //
    // Examples:
    // - "/" -> "/"
    // - "/a/b" -> "/a/"
    // - "/a/b/" -> "/a/b/"
    if path.ends_with('/') {
        return path;
    }
    match path.rfind('/') {
        Some(idx) => &path[..=idx],
        None => "/",
    }
}

fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            parts.pop();
            continue;
        }
        parts.push(seg);
    }

    let mut out = String::from("/");
    out.push_str(&parts.join("/"));
    if out.is_empty() {
        "/".to_string()
    } else {
        out
    }
}
