use near_sdk::FunctionError;

use thiserror::Error as ThisError;

#[cfg(feature = "contract")]
pub(crate) const ERR_UNAUTHORIZED_NOTIFIER_ID: &str = "unauthorized notifier_id";
#[cfg(feature = "contract")]
pub(crate) const ERR_UNAUTHORIZED_WAITER: &str = "unauthorized waiter";
#[cfg(feature = "contract")]
pub(crate) const ERR_ALREADY_WAITING: &str =
    "already waiting for notification - cv_wait is not intentded to be use concurrently";
#[cfg(feature = "contract")]
pub(crate) const ERR_ALREADY_DONE: &str = "already done - cv_wait succeeds only once";
#[cfg(feature = "contract")]
pub(crate) const ERR_ALREADY_NOTIFIED: &str =
    "already notified - contract is intentded to be notified exactly once";

#[derive(Debug, ThisError, FunctionError)]
pub enum Error {
    #[error("borsh: {0}")]
    Borsh(#[from] near_sdk::borsh::io::Error),

    #[error("cleanup in progress")]
    CleanupInProgress,
}
