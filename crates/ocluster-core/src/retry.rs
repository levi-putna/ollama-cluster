use serde::{Deserialize, Serialize};

/// Classification of upstream errors for retry decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryableError {
    ConnectionRefused,
    ConnectionTimeout,
    DnsFailure,
    NetworkUnreachable,
    HttpServerError,
    MalformedResponse,
    UpstreamDisconnectBeforeResponse,
    UpstreamDisconnectDuringStream,
    ClientCancellation,
    ApplicationError,
}

/// Retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub retry_timeout_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 2,
            retry_timeout_ms: 30_000,
        }
    }
}

impl RetryPolicy {
    /// Whether another retry attempt is allowed.
    pub fn allows_retry(&self, attempts: u32) -> bool {
        attempts < self.max_attempts
    }
}

/// Classify an error for retry and health tracking.
pub fn classify_error(status: Option<u16>, kind: &str) -> RetryableError {
    match kind {
        "connection_refused" => RetryableError::ConnectionRefused,
        "connection_timeout" => RetryableError::ConnectionTimeout,
        "dns_failure" => RetryableError::DnsFailure,
        "network_unreachable" => RetryableError::NetworkUnreachable,
        "malformed_response" => RetryableError::MalformedResponse,
        "upstream_disconnect_before" => RetryableError::UpstreamDisconnectBeforeResponse,
        "upstream_disconnect_stream" => RetryableError::UpstreamDisconnectDuringStream,
        "client_cancellation" => RetryableError::ClientCancellation,
        "application_error" => RetryableError::ApplicationError,
        _ if status.is_some_and(|s| s >= 500) => RetryableError::HttpServerError,
        _ => RetryableError::ApplicationError,
    }
}

impl RetryableError {
    /// Whether the proxy may retry on another node before streaming begins.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            RetryableError::ConnectionRefused
                | RetryableError::ConnectionTimeout
                | RetryableError::DnsFailure
                | RetryableError::NetworkUnreachable
                | RetryableError::HttpServerError
                | RetryableError::MalformedResponse
                | RetryableError::UpstreamDisconnectBeforeResponse
        )
    }

    /// Whether this should count as a node health failure.
    pub fn counts_as_node_failure(&self) -> bool {
        !matches!(
            self,
            RetryableError::ClientCancellation | RetryableError::ApplicationError
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Covers: FR-074–FR-076, TR-053, TXR-150-05
    #[test]
    fn connection_errors_are_retryable() {
        assert!(classify_error(None, "connection_refused").is_retryable());
        assert!(classify_error(Some(503), "http").is_retryable());
    }

    /// Covers: TR-091, TXR-120-07
    #[test]
    fn client_cancellation_not_node_failure() {
        let err = classify_error(None, "client_cancellation");
        assert!(!err.is_retryable());
        assert!(!err.counts_as_node_failure());
    }

    /// Covers: TR-091, TXR-120-08
    #[test]
    fn application_error_not_node_failure() {
        let err = classify_error(Some(400), "application_error");
        assert!(!err.counts_as_node_failure());
    }

    /// Covers: FR-076, TR-053
    #[test]
    fn retry_policy_respects_limit() {
        let policy = RetryPolicy {
            max_attempts: 2,
            retry_timeout_ms: 1000,
        };
        assert!(policy.allows_retry(0));
        assert!(policy.allows_retry(1));
        assert!(!policy.allows_retry(2));
    }
}
