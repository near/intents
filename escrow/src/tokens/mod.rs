#[cfg(feature = "nep141")]
mod nep141;
#[cfg(feature = "nep245")]
mod nep245;

use defuse_token_id::{TokenId, TokenIdType};
use derive_more::From;
use near_sdk::{AccountId, Gas, near};
use serde_with::{DisplayFromStr, serde_as};

use crate::{price::Price, state::Params};

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferMessage {
    pub params: Params,
    pub action: TransferAction,
}

#[near(serializers = [json])]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
#[derive(Debug, Clone, From)]
pub enum TransferAction {
    Open,
    Fill(FillAction),
    // Borrow(BorrowAction),
    // Repay(RepayAction),
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FillAction {
    pub price: Price,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub receive_src_to: OverrideSend,
    // TODO: price? for surplus
    // TODO: min_src_out?
}

// #[near(serializers = [json])]
// #[derive(Debug, Clone)]
// pub struct BorrowAction {

// }

// #[near(serializers = [json])]
// #[derive(Debug, Clone)]
// pub struct RepayAction {}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverrideSend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_id: Option<AccountId>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct Sent {
    pub token_type: TokenIdType,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub is_call: bool,
}

pub trait TokenIdExt: Sized {
    fn token_type(&self) -> TokenIdType;

    fn transfer_gas_min_default(&self, is_call: bool) -> (Gas, Gas);

    fn transfer_gas(&self, min_gas: Option<Gas>, is_call: bool) -> Gas {
        let (min, default) = self.transfer_gas_min_default(is_call);
        min_gas.unwrap_or(default).max(min)
    }
}

impl TokenIdExt for TokenId {
    #[inline]
    fn token_type(&self) -> TokenIdType {
        self.into()
    }

    fn transfer_gas_min_default(&self, is_call: bool) -> (Gas, Gas) {
        match self {
            #[cfg(feature = "nep141")]
            Self::Nep141(token) => token.transfer_gas_min_default(is_call),
            #[cfg(feature = "nep245")]
            Self::Nep245(token) => token.transfer_gas_min_default(is_call),
        }
    }
}
