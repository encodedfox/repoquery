use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidateError {
    #[error("Core error: {0}")]
    Core(#[from] od_core::CoreError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Validation error: {0}")]
    Other(String),
}
