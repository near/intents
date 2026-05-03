use tower::BoxError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("resolve: {0}")]
    Resolve(anyhow::Error),
    #[error(transparent)]
    Execute(BoxError),
}
