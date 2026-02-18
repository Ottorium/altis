#[derive(Debug)]
pub enum ApiError {
    Miscellaneous(String),
    Authentication(String),
    Parsing(String),
    Network(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Authentication(m) => write!(f, "Auth Error: {}", m),
            ApiError::Parsing(m) => write!(f, "Parsing Error: {}", m),
            ApiError::Network(m) => write!(f, "Network Error: {}", m),
            ApiError::Miscellaneous(m) => write!(f, "{}", m),
        }
    }
}

impl From<String> for ApiError {
    fn from(s: String) -> Self {
        ApiError::Miscellaneous(s)
    }
}

impl From<&str> for ApiError {
    fn from(s: &str) -> Self {
        ApiError::Miscellaneous(s.to_string())
    }
}
