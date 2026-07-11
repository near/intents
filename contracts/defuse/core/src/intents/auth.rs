use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, AccountIdRef, CryptoHash, Gas, NearToken, state_init::StateInit};
use serde::{Deserialize, Serialize};

use crate::{
    Result,
    engine::{Engine, Inspector, State},
    intents::ExecutableIntent,
};

/// Call `contract_id::on_auth(signer_id, msg)` with `signer_id`
/// of intent.
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema, ::borsh::BorshSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct AuthCall {
    /// Callee for `on_auth()`
    pub contract_id: AccountId,

    /// Optionally initialize the receiver's contract (Deterministic `AccountId`)
    /// via [`state_init`](https://github.com/near/NEPs/blob/master/neps/nep-0616.md#stateinit-action)
    /// right before calling `on_auth()` (in the same receipt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_init: Option<StateInit>,

    /// `msg` to pass in `on_auth()`
    pub msg: String,

    /// Optionally, attach deposit to `on_auth()`
    /// call. The amount will be subtracted from user's NEP-141 `wNEAR`
    /// balance.
    ///
    /// NOTE: the `wNEAR` will not be refunded in case of fail.
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub attached_deposit: NearToken,

    /// Optional minimum gas required for created promise to succeed.
    /// By default, only [`MIN_GAS_DEFAULT`](AuthCall::MIN_GAS_DEFAULT) is
    /// required.
    ///
    /// Remaining gas will be distributed evenly across all Function Call
    /// Promises created during execution of current receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl AuthCall {
    pub const MIN_GAS_DEFAULT: Gas = Gas::from_tgas(10);

    #[inline]
    pub fn min_gas(&self) -> Gas {
        self.min_gas.unwrap_or(Self::MIN_GAS_DEFAULT)
    }
}

impl ExecutableIntent for AuthCall {
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        _intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine.state.auth_call(signer_id, self)
    }
}
