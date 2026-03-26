use crate::api::types::ApiError;
use thiserror::Error;

/// Custom error types for the Board API client
#[derive(Debug, Error)]
pub enum BoardApiError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error: {message} (status: {status})")]
    Api { message: String, status: u16 },

    /// Failed to parse response
    #[error("Failed to parse response: {0}")]
    Parse(String),

    /// Invalid URL format
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Device code not set
    #[error("Device code not set - call register_device() first")]
    NoDeviceCode,

    /// Invalid paste ID format
    #[error("Invalid paste ID: {0}")]
    InvalidPasteId(String),

    /// Content too large
    #[error("Content exceeds maximum size limit")]
    ContentTooLarge,

    /// Rate limit exceeded
    #[error("Rate limit exceeded - try again later")]
    RateLimited,

    /// Network timeout
    #[error("Request timed out")]
    Timeout,
}

impl From<ApiError> for BoardApiError {
    fn from(api_error: ApiError) -> Self {
        Self::Api {
            message: api_error.error,
            status: api_error.status,
        }
    }
}

/// Result type for Board API operations
pub type ApiResult<T> = std::result::Result<T, BoardApiError>;