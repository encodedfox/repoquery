use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Core error: {0}")]
    Core(#[from] rq_core::CoreError),
    #[error("Graph error: {0}")]
    Other(String),
}
