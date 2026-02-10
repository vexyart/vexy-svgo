use thiserror::Error;

#[derive(Debug, Error)]
pub enum VexyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("XML parsing error: {0}")]
    Parse(#[from] crate::parser::error::ParseError),

    #[error("Plugin '{0}' failed: {1}")]
    Plugin(String, String),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("Regex error: {0}")]
    Regex(String),

    #[error("Memory budget exceeded: {0}")]
    Memory(#[from] crate::optimizer::memory::MemoryError),

    #[error("{0}")]
    General(String),
}

impl From<String> for VexyError {
    fn from(s: String) -> Self {
        VexyError::General(s)
    }
}

impl From<&str> for VexyError {
    fn from(s: &str) -> Self {
        VexyError::General(s.to_string())
    }
}

impl From<anyhow::Error> for VexyError {
    fn from(err: anyhow::Error) -> Self {
        VexyError::General(err.to_string())
    }
}

impl From<regex::Error> for VexyError {
    fn from(err: regex::Error) -> Self {
        VexyError::Regex(err.to_string())
    }
}

impl From<std::fmt::Error> for VexyError {
    fn from(err: std::fmt::Error) -> Self {
        VexyError::General(err.to_string())
    }
}
