mod tokens;
mod upgrade;
mod utils;

use crate::CondVarContext;
#[cfg(feature = "escrow-swap")]
use defuse_escrow_swap::ext_escrow;
use defuse_near_utils::UnwrapOrPanicError;
#[cfg(feature = "escrow-swap")]
use near_sdk::state_init::{StateInit, StateInitV1};
use near_sdk::{
    AccountId, CryptoHash, Gas, PanicOnDefault, Promise, env, json_types::U128, near, require,
};

use crate::EscrowProxy;
use crate::message::TransferMessage;
use crate::state::ProxyConfig;
#[cfg(feature = "escrow-swap")]
use defuse_escrow_swap::Params as EscrowParams;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    config: ProxyConfig,
}

#[near]
impl Contract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(config: ProxyConfig) -> Contract {
        Self { config }
    }

    fn assert_owner(&self) {
        require!(
            env::predecessor_account_id() == self.config.owner_id,
            "Only owner can call this method"
        );
    }
}

#[near]
impl EscrowProxy for Contract {
    /// Returns proxy configuration
    fn config(&self) -> &ProxyConfig {
        &self.config
    }

    /// Calculates oneshot condvar context hash that is required to derive condvar contract
    /// instance address
    fn context_hash(&self, context: CondVarContext<'static>) -> CryptoHash {
        context.hash()
    }

    /// Calculates oneshot condvar contract instance address, helper function for integration
    /// purposes, and easy calculation of oneshot condvar contract instance address in case of
    /// need for direct authorization using OneshotCondvar::cv_notify_one
    /// taker_id: The account id of the taker
    /// token_ids: The token ids of the tokens being transferred
    /// amounts: The amounts of the tokens being transferred
    /// msg: escrow proxy transfer message
    fn oneshot_address(
        &self,
        taker_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> AccountId {
        use std::borrow::Cow;
        let transfer_message: TransferMessage = msg.parse().unwrap_or_panic_display();
        let context_hash = CondVarContext {
            sender_id: Cow::Owned(taker_id),
            token_ids: Cow::Owned(token_ids),
            amounts: Cow::Owned(amounts),
            salt: transfer_message.salt,
            msg: Cow::Borrowed(&msg),
        }
        .hash();
        let auth_contract_state_init =
            self.get_deterministic_transfer_auth_state_init(context_hash);
        auth_contract_state_init.derive_account_id()
    }
}

#[cfg(feature = "escrow-swap")]
#[near]
impl Contract {
    /// Calculates escrow contract instance address
    pub fn escrow_address(&self, params: &EscrowParams) -> AccountId {
        let raw_state = defuse_escrow_swap::ContractStorage::init_state(params)
            .unwrap_or_else(|e| env::panic_str(&format!("Invalid escrow params: {e}")));
        let state_init = StateInit::V1(StateInitV1 {
            code: self.config.escrow_swap_contract_id.clone(),
            data: raw_state,
        });
        state_init.derive_account_id()
    }

    pub fn cancel_escrow(&self, params: EscrowParams) -> Promise {
        self.assert_owner();
        let escrow_address = self.escrow_address(&params);
        ext_escrow::ext(escrow_address)
            .with_static_gas(Gas::from_tgas(50))
            .es_close(params)
    }
}
