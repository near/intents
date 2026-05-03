#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("code hash mismatch")]
    CodeHashMismatch,

    #[error("NEAR: {0}")]
    NearRpc(#[from] near_kit::Error),

    #[error("URL: {0}")]
    Url(#[from] super::url::Error),
}
