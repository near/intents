use near_sdk::{AccountId, env};
use std::sync::LazyLock;

/// Cached [`env::current_account_id()`]
pub static CURRENT_ACCOUNT_ID: LazyLock<AccountId> = LazyLock::new(env::current_account_id);
/// Cached [`env::predecessor_account_id()`]
pub static PREDECESSOR_ACCOUNT_ID: LazyLock<AccountId> = LazyLock::new(env::predecessor_account_id);

#[cfg(feature = "time")]
/// Cached [`env::block_timestamp()`]
pub static BLOCK_TIMESTAMP: LazyLock<chrono::DateTime<chrono::Utc>> =
    LazyLock::new(crate::time::now);
