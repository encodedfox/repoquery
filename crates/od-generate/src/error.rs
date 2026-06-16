use thiserror::Error;

#[derive(Debug, Error)]
pub enum GenerateError {
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Core error: {0}")]
    Core(#[from] od_core::CoreError),
}
