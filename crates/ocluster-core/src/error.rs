use thiserror::Error;

/// Domain-level errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("no eligible nodes for model '{model}'")]
    NoEligibleNodes { model: String },
}
