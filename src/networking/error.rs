use std::fmt;

#[derive(Debug)]
pub enum NetworkError {
    InvalidUri,
    ConnectionFailed(String),
    SendFailed(String),
    ReceiveFailed(String),
    TlsError(String),
    MissingMethod,
    MissingUri,
    ParseError(String),
    HeaderParseError(String),
    InvalidHttpVersion,
    InvalidStatusCode,
    InvalidHeader,
    TooLargeResponse,
    TooManyRedirects,
    Timeout,
}

impl std::error::Error for NetworkError {}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::InvalidUri => write!(f, "Invalid URI"),
            NetworkError::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            NetworkError::SendFailed(e) => write!(f, "Failed to send request: {}", e),
            NetworkError::ReceiveFailed(e) => write!(f, "Failed to receive response: {}", e),
            NetworkError::TlsError(e) => write!(f, "TLS error: {}", e),
            NetworkError::MissingMethod => write!(f, "HTTP method not specified"),
            NetworkError::MissingUri => write!(f, "URI not specified"),
            NetworkError::ParseError(e) => write!(f, "Parse error: {}", e),
            NetworkError::HeaderParseError(e) => write!(f, "Header parse error: {}", e),
            NetworkError::InvalidHttpVersion => write!(f, "Invalid HTTP version"),
            NetworkError::InvalidStatusCode => write!(f, "Invalid status code"),
            NetworkError::InvalidHeader => write!(f, "Invalid header"),
            NetworkError::TooLargeResponse => write!(f, "Response too large"),
            NetworkError::TooManyRedirects => write!(f, "Too many redirects"),
            NetworkError::Timeout => write!(f, "Request timed out"),
        }
    }
}
