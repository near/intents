use std::sync::LazyLock;

use near_sdk::{AccountId, env};

/// Cached [`env::current_account_id()`]
pub static CURRENT_ACCOUNT_ID: LazyLock<AccountId> = LazyLock::new(env::current_account_id);
/// Cached [`env::predecessor_account_id()`]
pub static PREDECESSOR_ACCOUNT_ID: LazyLock<AccountId> = LazyLock::new(env::predecessor_account_id);
