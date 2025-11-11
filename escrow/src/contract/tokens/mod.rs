#[cfg(feature = "nep141")]
mod nep141;
#[cfg(feature = "nep245")]
mod nep245;

const _: () = assert!(
    cfg!(any(feature = "nep141", feature = "nep245")),
    "at least one of these features should be enabled: ['nep141', 'nep245']"
);

use defuse_token_id::{TokenId, TokenIdType};
use near_sdk::{AccountId, Gas, Promise, PromiseResult, env, json_types::U128, near, serde_json};

use serde_with::{DisplayFromStr, serde_as};

pub trait Sendable {
    fn send(
        self,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> Promise;
}

impl Sendable for TokenId {
    fn send(
        self,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> Promise {
        match self {
            #[cfg(feature = "nep141")]
            Self::Nep141(token) => token.send(receiver_id, amount, memo, msg, min_gas, unused_gas),
            #[cfg(feature = "nep245")]
            Self::Nep245(token) => token.send(receiver_id, amount, memo, msg, min_gas, unused_gas),
        }
    }
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
pub struct SentAsset {
    pub asset_type: TokenIdType,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub is_call: bool,
}

impl SentAsset {
    // TODO
    /// Returns refund
    pub fn resolve_refund(&self, result_idx: u64) -> u128 {
        self.amount.saturating_sub(self.resolve_used(result_idx))
    }

    fn resolve_used(&self, result_idx: u64) -> u128 {
        match env::promise_result(result_idx) {
            PromiseResult::Successful(value) => {
                if self.is_call {
                    match self.asset_type {
                        #[cfg(feature = "nep141")]
                        TokenIdType::Nep141 => {
                            // `ft_transfer_call` returns successfully transferred amount

                            serde_json::from_slice::<U128>(&value).unwrap_or_default().0
                        }
                        #[cfg(feature = "nep245")]
                        TokenIdType::Nep245 => {
                            // `ft_transfer_call` returns successfully transferred amount
                            serde_json::from_slice::<[U128; 1]>(&value).unwrap_or_default()[0].0
                        }
                    }
                    .min(self.amount)
                } else if value.is_empty() {
                    // `ft_transfer` returns empty result on success
                    self.amount
                } else {
                    0
                }
            }
            PromiseResult::Failed => {
                if self.is_call {
                    // do not refund on failed `ft_transfer_call` due to
                    // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                    // read result of `ft_on_transfer` due to insufficient gas
                    self.amount
                } else {
                    0
                }
            }
        }
    }
}

// pub trait TokenType {
//     fn parse_transfer_ok(&self, data: &[u8]) -> bool;
//     fn parse_transfer_call_ok(&self, data: &[u8]) -> u128;

//     fn transfer_call_failed_refund(&self) -> bool;

//     fn resolve_transfer(&self, result_idx: u64, amount: u128, is_call: bool) -> u128 {
//         match env::promise_result(result_idx) {
//             PromiseResult::Successful(data) => {
//                 if is_call {
//                     self.parse_transfer_call_ok(&data).min(amount)
//                 } else if self.parse_transfer_ok(&data) {
//                     amount
//                 } else {
//                     0
//                 }
//             }
//             PromiseResult::Failed => {
//                 if is_call {
//                     // do not refund on failed `mt_transfer_call` due to
//                     // NEP-245 vulnerability: `mt_resolve_transfer` fails to
//                     // read result of `mt_on_transfer` due to insufficient gas
//                     amount
//                 } else {
//                     0
//                 }
//             }
//         }
//     }
// }
