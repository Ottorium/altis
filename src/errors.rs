#[derive(Debug)]
pub enum UntisError {
    Miscellaneous(String),
    Authentication(String),
    Parsing(String),
    Network(String),
}

impl std::fmt::Display for UntisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UntisError::Authentication(m) => write!(f, "Auth Error: {}", m),
            UntisError::Parsing(m) => write!(f, "Parsing Error: {}", m),
            UntisError::Network(m) => write!(f, "Network Error: {}", m),
            UntisError::Miscellaneous(m) => write!(f, "{}", m),
        }
    }
}

impl From<String> for UntisError {
    fn from(s: String) -> Self {
        UntisError::Miscellaneous(s)
    }
}

impl From<&str> for UntisError {
    fn from(s: &str) -> Self {
        UntisError::Miscellaneous(s.to_string())
    }
}
