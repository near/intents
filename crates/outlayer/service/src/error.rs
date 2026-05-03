use defuse_outlayer_executor as executor;

use crate::resolver;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("resolve: {0}")]
    Resolve(resolver::Error),
    #[error(transparent)]
    Execute(executor::Error),
}
