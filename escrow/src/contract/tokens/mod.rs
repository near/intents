#[cfg(feature = "nep141")]
mod nep141;
#[cfg(feature = "nep245")]
mod nep245;

use near_sdk::{AccountId, Gas, Promise};

pub trait Token {
    fn send(
        self,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> Promise;

    // Returns actually transferred amount of a single token.
    fn resolve(result_idx: u64, amount: u128, is_call: bool) -> u128;
}
