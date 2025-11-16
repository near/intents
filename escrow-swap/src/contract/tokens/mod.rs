#[cfg(feature = "nep141")]
mod nep141;
#[cfg(feature = "nep245")]
mod nep245;

use defuse_token_id::{TokenId, TokenIdType};
use near_sdk::{
    AccountId, Gas, Promise, PromiseOrValue, PromiseResult, env, json_types::U128, serde_json,
};

use crate::{
    Error, Params, Result, State,
    action::{TransferAction, TransferMessage},
    tokens::{Sent, TokenIdExt},
};

use super::Contract;

impl Contract {
    /// Returns amount to be refunded
    pub fn on_receive(
        &mut self,
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<PromiseOrValue<u128>> {
        if amount == 0 {
            return Err(Error::InsufficientAmount);
        }

        let msg: TransferMessage = serde_json::from_str(msg)?;

        self.cleanup_guard()
            .try_as_alive_mut()?
            .verify_mut(&msg.params)?
            .on_receive(msg.params, sender_id, token_id, amount, msg.action)
    }
}

impl State {
    /// Returns amount to be refunded
    fn on_receive(
        &mut self,
        params: Params,
        // TODO: it could have been EscrowFactory who forwarded funds to us
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        action: TransferAction,
    ) -> Result<PromiseOrValue<u128>> {
        if self.closed || self.deadline.has_expired() {
            return Err(Error::Closed);
        }

        match action {
            TransferAction::Fund if token_id == params.src_token => {
                self.on_fund(params, sender_id, amount)
            }
            TransferAction::Fill(fill) if token_id == params.dst_token => {
                self.on_fill(params, sender_id, amount, fill)
            }
            _ => Err(Error::WrongToken),
        }
    }
}

pub trait Sendable: TokenIdExt + Sized {
    fn send(
        self,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> Promise;

    fn send_for_resolve(
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
