use serde::{Deserialize, Serialize};

/// Circuit breaker states per node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    #[default]
    Closed,
    Open,
    HalfOpen,
}

/// Per-node circuit breaker tracking consecutive failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub consecutive_successes: u32,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given thresholds.
    pub fn new(failure_threshold: u32, success_threshold: u32) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            success_threshold,
            consecutive_successes: 0,
        }
    }

    /// Whether the node is eligible for routing.
    pub fn allows_routing(&self) -> bool {
        !matches!(self.state, CircuitState::Open)
    }

    /// Record a successful request or probe.
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        match self.state {
            CircuitState::HalfOpen => {
                self.consecutive_successes += 1;
                if self.consecutive_successes >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.consecutive_successes = 0;
                }
            }
            CircuitState::Open => {
                self.state = CircuitState::HalfOpen;
                self.consecutive_successes = 1;
            }
            CircuitState::Closed => {}
        }
    }

    /// Record a failure from upstream.
    pub fn record_failure(&mut self) {
        self.consecutive_successes = 0;
        self.failure_count += 1;
        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
    }

    /// Begin recovery probing (half-open).
    pub fn begin_recovery(&mut self) {
        if self.state == CircuitState::Open {
            self.state = CircuitState::HalfOpen;
            self.consecutive_successes = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Covers: FR-031, FR-032, TR-092, TXR-120-03
    #[test]
    fn opens_after_threshold_failures() {
        let mut cb = CircuitBreaker::new(3, 2);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);
        assert!(!cb.allows_routing());
    }

    /// Covers: TR-092, TXR-120-05
    #[test]
    fn recovers_after_success_threshold() {
        let mut cb = CircuitBreaker::new(2, 2);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);
        cb.begin_recovery();
        assert_eq!(cb.state, CircuitState::HalfOpen);
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state, CircuitState::Closed);
        assert!(cb.allows_routing());
    }
}
