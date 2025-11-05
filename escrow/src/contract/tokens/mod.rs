#[cfg(feature = "nep141")]
mod nep141;
#[cfg(feature = "nep245")]
mod nep245;

use defuse_token_id::TokenId;
use near_sdk::{AccountId, Gas, Promise};

use crate::contract::Contract;

pub trait Sender<A> {
    fn send(
        asset: A,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
    ) -> Promise;
}

impl Sender<TokenId> for Contract {
    fn send(
        asset: TokenId,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
    ) -> Promise {
        match asset {
            #[cfg(feature = "nep141")]
            TokenId::Nep141(asset) => Self::send(asset, receiver_id, amount, memo, msg, min_gas),
            #[cfg(feature = "nep245")]
            TokenId::Nep245(asset) => Self::send(asset, receiver_id, amount, memo, msg, min_gas),
        }
    }
}
