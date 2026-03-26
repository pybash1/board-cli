use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a device code used for authentication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCode(pub String);

impl DeviceCode {
    /// Create a new device code from a string
    pub fn new(code: String) -> Self {
        Self(code)
    }

    /// Get the device code as a string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for DeviceCode {
    fn from(code: String) -> Self {
        Self(code)
    }
}

impl From<&str> for DeviceCode {
    fn from(code: &str) -> Self {
        Self(code.to_string())
    }
}

/// Represents a paste ID
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteId(pub String);

impl PasteId {
    /// Create a new paste ID from a string
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Get the paste ID as a string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PasteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for PasteId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for PasteId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Represents a paste stored on the Board API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paste {
    /// The unique identifier for this paste
    pub id: PasteId,
    /// The content of the paste
    pub content: String,
    /// The URL to access this paste
    pub url: String,
}

impl Paste {
    /// Create a new paste instance
    pub fn new(id: PasteId, content: String, url: String) -> Self {
        Self { id, content, url }
    }
}

/// API information response from the root endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    /// Service description message
    pub message: String,
    /// Available API endpoints
    pub endpoints: Vec<EndpointInfo>,
}

/// Information about a single API endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointInfo {
    /// HTTP method (GET, POST, PUT, etc.)
    pub method: String,
    /// URL path for this endpoint
    pub path: String,
    /// Description of what this endpoint does
    pub description: String,
}

/// Error response from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error message
    pub error: String,
    /// HTTP status code
    pub status: u16,
}