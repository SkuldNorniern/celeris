use crate::networking::error::NetworkError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Response {
    pub version: Version,
    pub status: Status,
    pub headers: Headers,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub struct Request {
    method: Method,
    uri: String,
    version: Version,
    headers: Headers,
    body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Headers(HashMap<String, String>);

#[derive(Debug)]
pub enum Method {
    GET,
    POST,
    HEAD,
    // Add more as needed
}

#[derive(Debug, Clone)]
pub struct Status {
    pub code: u16,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum Version {
    Http10,
    Http11,
}

impl Request {
    pub fn new() -> RequestBuilder {
        RequestBuilder::new()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut request = Vec::new();

        // Request line
        let method = match self.method {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::HEAD => "HEAD",
        };

        let version = match self.version {
            Version::Http10 => "HTTP/1.0",
            Version::Http11 => "HTTP/1.1",
        };

        request.extend(format!("{} {} {}\r\n", method, self.uri, version).as_bytes());

        // Headers
        for (name, value) in self.headers.iter() {
            request.extend(format!("{}: {}\r\n", name, value).as_bytes());
        }

        // Empty line separating headers from body
        request.extend(b"\r\n");

        // Body
        request.extend(&self.body);

        request
    }
}

pub struct RequestBuilder {
    method: Option<Method>,
    uri: Option<String>,
    headers: Headers,
    body: Vec<u8>,
}

impl RequestBuilder {
    fn new() -> Self {
        Self {
            method: None,
            uri: None,
            headers: Headers::new(),
            body: Vec::new(),
        }
    }

    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    pub fn uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn build(self) -> Result<Request, NetworkError> {
        Ok(Request {
            method: self.method.ok_or(NetworkError::MissingMethod)?,
            uri: self.uri.ok_or(NetworkError::MissingUri)?,
            version: Version::Http11,
            headers: self.headers,
            body: self.body,
        })
    }
}

impl Headers {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, name: String, value: String) {
        self.0.insert(name.to_lowercase(), value);
    }

    pub fn get(&self, name: &str) -> Option<&String> {
        self.0.get(&name.to_lowercase())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.0.iter()
    }
}
