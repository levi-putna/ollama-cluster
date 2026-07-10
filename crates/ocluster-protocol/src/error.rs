use serde::{Deserialize, Serialize};

/// Structured management API error response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiErrorBody {
    pub error: ApiErrorDetail,
}

/// Error detail payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiErrorBody {
    /// Create a new API error body.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ApiErrorDetail {
                code: code.into(),
                message: message.into(),
                details: None,
            },
        }
    }
}
