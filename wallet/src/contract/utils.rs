use std::fmt::Display;

use near_sdk::env;

pub fn panic_msg(msg: impl Display) -> ! {
    env::panic_str(&msg.to_string())
}

// TODO: remove in favor of `env::chain_id()` when NEP-638 lands
pub fn chain_id() -> String {
    "mainnet".to_string()
}
