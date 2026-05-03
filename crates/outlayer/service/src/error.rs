use defuse_outlayer_executor as executor;

#[derive(thiserror::Error, Debug)]
pub enum Error<R> {
    #[error("resolve: {0}")]
    Resolve(R),
    #[error(transparent)]
    Execute(executor::Error),
}
