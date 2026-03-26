pub mod client;
pub mod error;
pub mod types;

// Re-export the main types for convenience
pub use client::{BoardClient, BoardClientConfig};
pub use error::{BoardApiError, ApiResult};
pub use types::{ApiInfo, DeviceCode, Paste, PasteId, EndpointInfo};