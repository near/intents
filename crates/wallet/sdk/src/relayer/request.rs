use defuse_wallet::{RequestMessage, StateInit};
use serde::{Deserialize, Serialize};

use crate::Proof;

/// Request for [relaying](super::WalletRelayer) messages
/// to [`w_execute_signed()`](crate::contract::Wallet::w_execute_signed)
/// method on [`msg.signer_id`](field@RequestMessage::signer_id) contract.
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletRelayRequest {
    /// Optionally initialize the contract before calling
    /// [`w_execute_signed()`](crate::contract::Wallet::w_execute_signed).
    ///
    /// This is only required for the first transaction. If the contract is
    /// already initialized, this is no-op.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deterministic_state_init: Option<StateInit>,

    /// Signed message
    pub msg: RequestMessage,

    /// Proof (i.e. signature)
    pub proof: Proof,
    // TODO: gas hint?
}

impl WalletRelayRequest {
    #[must_use]
    #[inline]
    pub fn new(msg: RequestMessage, proof: impl Into<String>) -> Self {
        Self {
            deterministic_state_init: None,
            msg,
            proof: proof.into(),
        }
    }

    #[must_use]
    #[inline]
    pub fn deterministic_state_init(mut self, state_init: impl Into<StateInit>) -> Self {
        self.deterministic_state_init = Some(state_init.into());
        self
    }
}
