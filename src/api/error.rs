use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxmoxError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Unauthorized — check token")]
    Unauthorized,
    #[error("Forbidden — insufficient permissions")]
    Forbidden,
}
