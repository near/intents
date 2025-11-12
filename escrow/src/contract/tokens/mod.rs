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

pub trait TokenIdExt: Sized {
    fn token_type(&self) -> TokenIdType;

    fn send(
        self,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> Promise;

    fn send_to_resolve_later(
        self,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> (Sent, Promise) {
        (
            Sent {
                token_type: self.token_type(),
                amount,
                is_call: msg.is_some(),
            },
            self.send(receiver_id, amount, memo, msg, min_gas, unused_gas),
        )
    }
}

impl TokenIdExt for TokenId {
    #[inline]
    fn token_type(&self) -> TokenIdType {
        self.into()
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct Sent {
    pub token_type: TokenIdType,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub is_call: bool,
}

impl Sent {
    /// Returns refund
    #[must_use]
    pub fn resolve_refund(&self, result_idx: u64) -> u128 {
        self.amount.saturating_sub(self.resolve_used(result_idx))
    }

    #[must_use]
    fn resolve_used(&self, result_idx: u64) -> u128 {
        match env::promise_result(result_idx) {
            PromiseResult::Successful(value) => {
                if self.is_call {
                    self.token_type.parse_transfer_call(value).min(self.amount)
                } else if value.is_empty() {
                    // `{ft,mt}_transfer` returns empty result on success
                    self.amount
                } else {
                    0
                }
            }
            PromiseResult::Failed => {
                if self.is_call {
                    // do not refund on failed `{ft,mt}_transfer_call` due to
                    // vulnerability in reference implementations of FT and MT:
                    // `{ft,mt}_resolve_transfer` can fail to huge read result
                    // returned by `{ft,mt}_on_transfer` due to insufficient gas
                    self.amount
                } else {
                    0
                }
            }
        }
    }
}

pub trait TokenIdTypeExt {
    fn refund_value(&self, refund: u128) -> serde_json::Result<Vec<u8>>;
    fn parse_transfer_call(&self, value: Vec<u8>) -> u128;
}

impl TokenIdTypeExt for TokenIdType {
    fn refund_value(&self, refund: u128) -> serde_json::Result<Vec<u8>> {
        match self {
            #[cfg(feature = "nep141")]
            TokenIdType::Nep141 => serde_json::to_vec(&U128(refund)),
            #[cfg(feature = "nep245")]
            TokenIdType::Nep245 => serde_json::to_vec(&[U128(refund)]),
        }
    }

    fn parse_transfer_call(&self, value: Vec<u8>) -> u128 {
        match self {
            #[cfg(feature = "nep141")]
            Self::Nep141 => {
                // `ft_transfer_call` returns successfully transferred amount
                serde_json::from_slice::<U128>(&value).unwrap_or_default().0
            }
            #[cfg(feature = "nep245")]
            Self::Nep245 => {
                // `mt_transfer_call` returns successfully transferred amounts
                serde_json::from_slice::<[U128; 1]>(&value).unwrap_or_default()[0].0
            }
        }
    }
}
