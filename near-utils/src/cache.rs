use near_sdk::{AccountId, env};
use std::sync::LazyLock;

#[cfg(feature = "time")]
use chrono::{DateTime, Utc};

/// Cached [`env::current_account_id()`]
pub static CURRENT_ACCOUNT_ID: LazyLock<AccountId> = LazyLock::new(env::current_account_id);
/// Cached [`env::predecessor_account_id()`]
pub static PREDECESSOR_ACCOUNT_ID: LazyLock<AccountId> = LazyLock::new(env::predecessor_account_id);

/// Cached [`env::block_timestamp()`]
#[cfg(feature = "time")]
pub static BLOCK_TIMESTAMP: LazyLock<DateTime<Utc>> = LazyLock::new(crate::time::now);
