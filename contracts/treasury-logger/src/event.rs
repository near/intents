use defuse_nep245::TokenId;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, near};

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "logger"))]
pub enum Event {
    #[event_version("1.0.0")]
    MtDeposit {
        token: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
        nonce: U128,
    },
}
