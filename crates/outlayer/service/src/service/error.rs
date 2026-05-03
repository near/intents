use crate::executor;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("resolver: {0}")]
    Resolver(anyhow::Error),
    #[error(transparent)]
    Executor(executor::Error),
}
