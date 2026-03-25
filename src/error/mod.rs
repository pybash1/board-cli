use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Terminal error: {0}")]
    Terminal(String),
}

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, AppError>;