use crate::api::error::{BoardApiError, ApiResult};
use crate::api::types::{ApiInfo, DeviceCode, Paste, PasteId};
use reqwest::{Client, Response};
use std::time::Duration;
use url::Url;

/// Configuration for the Board API client
#[derive(Debug, Clone)]
pub struct BoardClientConfig {
    /// Base URL for the API (default: https://board-api.pybash.xyz)
    pub base_url: String,
    /// Request timeout (default: 30 seconds)
    pub timeout: Duration,
    /// User agent string for requests
    pub user_agent: String,
    /// Device code for authentication (optional - can be set later)
    pub device_code: Option<DeviceCode>,
}

impl Default for BoardClientConfig {
    fn default() -> Self {
        Self {
            base_url: "https://board-api.pybash.xyz".to_string(),
            timeout: Duration::from_secs(30),
            user_agent: "board-cli/0.1.0".to_string(),
            device_code: None,
        }
    }
}

/// The main Board API client
#[derive(Debug, Clone)]
pub struct BoardClient {
    client: Client,
    config: BoardClientConfig,
}

impl BoardClient {
    /// Create a new Board API client with default configuration
    pub fn new() -> ApiResult<Self> {
        Self::with_config(BoardClientConfig::default())
    }

    /// Create a new Board API client with custom configuration
    pub fn with_config(config: BoardClientConfig) -> ApiResult<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()
            .map_err(BoardApiError::Request)?;

        Ok(Self { client, config })
    }

    /// Register a new device and get a device code for authentication
    pub async fn register_device(&mut self) -> ApiResult<DeviceCode> {
        let url = format!("{}/device", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(BoardApiError::Request)?;

        let device_code_str = self.handle_text_response(response).await?;
        let device_code = DeviceCode::new(device_code_str.trim().to_string());

        // Store the device code for future requests
        self.config.device_code = Some(device_code.clone());

        Ok(device_code)
    }

    /// Set the device code for authentication
    pub fn set_device_code(&mut self, device_code: DeviceCode) {
        self.config.device_code = Some(device_code);
    }

    /// Get the current device code if set
    pub fn device_code(&self) -> Option<&DeviceCode> {
        self.config.device_code.as_ref()
    }

    /// Get API information from the root endpoint
    pub async fn get_api_info(&self) -> ApiResult<ApiInfo> {
        let url = format!("{}/", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(BoardApiError::Request)?;

        self.handle_json_response(response).await
    }

    /// Create a new paste with raw content
    pub async fn create_paste(&self, content: &str) -> ApiResult<Paste> {
        let device_code = self.config.device_code.as_ref()
            .ok_or(BoardApiError::NoDeviceCode)?;

        let url = format!("{}/", self.config.base_url);

        let response = self
            .client
            .put(&url)
            .header("Device-Code", device_code.as_str())
            .header("Content-Type", "text/plain")
            .body(content.to_string())
            .send()
            .await
            .map_err(BoardApiError::Request)?;

        let response_text = self.handle_text_response(response).await?;

        // Parse the response URL to extract paste ID
        let paste_url = response_text.trim();
        let url_parsed = Url::parse(paste_url)?;
        let paste_id = url_parsed.path_segments()
            .and_then(|segments| segments.last())
            .ok_or_else(|| BoardApiError::Parse("Could not extract paste ID from URL".to_string()))?;

        Ok(Paste::new(
            PasteId::new(paste_id.to_string()),
            content.to_string(),
            paste_url.to_string(),
        ))
    }

    /// Get the content of a paste by its ID
    pub async fn get_paste(&self, paste_id: &PasteId) -> ApiResult<String> {
        let device_code = self.config.device_code.as_ref()
            .ok_or(BoardApiError::NoDeviceCode)?;

        let url = format!("{}/{}", self.config.base_url, paste_id.as_str());

        let response = self
            .client
            .get(&url)
            .header("Device-Code", device_code.as_str())
            .send()
            .await
            .map_err(BoardApiError::Request)?;

        self.handle_text_response(response).await
    }

    /// Get all paste IDs associated with the current device
    pub async fn list_pastes(&self) -> ApiResult<Vec<PasteId>> {
        let device_code = self.config.device_code.as_ref()
            .ok_or(BoardApiError::NoDeviceCode)?;

        let url = format!("{}/all", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .header("Device-Code", device_code.as_str())
            .send()
            .await
            .map_err(BoardApiError::Request)?;

        let paste_ids: Vec<String> = self.handle_json_response(response).await?;
        Ok(paste_ids.into_iter().map(PasteId::new).collect())
    }

    /// Get all pastes (IDs and content) for the current device
    pub async fn get_all_pastes(&self) -> ApiResult<Vec<Paste>> {
        let paste_ids = self.list_pastes().await?;
        let mut pastes = Vec::new();

        for paste_id in paste_ids {
            match self.get_paste(&paste_id).await {
                Ok(content) => {
                    let url = format!("{}/{}", self.config.base_url, paste_id.as_str());
                    pastes.push(Paste::new(paste_id, content, url));
                }
                Err(e) => {
                    // Log the error but continue with other pastes
                    eprintln!("Failed to fetch paste {}: {}", paste_id, e);
                }
            }
        }

        Ok(pastes)
    }

    /// Build a full URL for a paste ID
    pub fn build_paste_url(&self, paste_id: &PasteId) -> String {
        format!("{}/{}", self.config.base_url, paste_id.as_str())
    }

    /// Handle a JSON response from the API
    async fn handle_json_response<T>(&self, response: Response) -> ApiResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();
        let response_text = response.text().await.map_err(BoardApiError::Request)?;

        if !status.is_success() {
            // Try to parse as API error first
            if let Ok(api_error) = serde_json::from_str::<crate::api::types::ApiError>(&response_text) {
                return Err(BoardApiError::Api {
                    message: api_error.error,
                    status: api_error.status,
                });
            }

            // Fallback to generic error
            return Err(BoardApiError::Api {
                message: response_text,
                status: status.as_u16(),
            });
        }

        serde_json::from_str(&response_text)
            .map_err(|e| BoardApiError::Parse(format!("JSON parse error: {} - Response: {}", e, response_text)))
    }

    /// Handle a text response from the API
    async fn handle_text_response(&self, response: Response) -> ApiResult<String> {
        let status = response.status();
        let response_text = response.text().await.map_err(BoardApiError::Request)?;

        if !status.is_success() {
            // Try to parse as API error first
            if let Ok(api_error) = serde_json::from_str::<crate::api::types::ApiError>(&response_text) {
                return Err(BoardApiError::Api {
                    message: api_error.error,
                    status: api_error.status,
                });
            }

            // Fallback to generic error
            return Err(BoardApiError::Api {
                message: response_text,
                status: status.as_u16(),
            });
        }

        Ok(response_text)
    }
}

impl Default for BoardClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default BoardClient")
    }
}