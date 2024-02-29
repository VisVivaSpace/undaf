//! Main Crate Error


#[derive(thiserror::Error, Debug)]
pub enum DAFError {
    #[error("Generic {0}")]
    Generic(String), //TODO remove later

    #[error(transparent)]
    IO(#[from] std::io::Error),
}