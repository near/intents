use defuse_wallet_core::RequestMessage;
use near_kit::{Gas, StateInit};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletRelayRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_init: Option<StateInit>, // TODO
    pub msg: RequestMessage,
    pub proof: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas: Option<Gas>,
}

impl WalletRelayRequest {
    #[must_use]
    #[inline]
    pub fn new(msg: RequestMessage, proof: impl Into<String>) -> Self {
        Self {
            state_init: None,
            msg,
            proof: proof.into(),
            gas: None,
        }
    }

    #[must_use]
    #[inline]
    pub fn state_init(mut self, state_init: impl Into<StateInit>) -> Self {
        self.state_init = Some(state_init.into());
        self
    }

    #[must_use]
    #[inline]
    pub const fn gas(mut self, gas: Gas) -> Self {
        self.gas = Some(gas);
        self
    }
}
