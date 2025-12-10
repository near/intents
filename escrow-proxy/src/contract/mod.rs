mod admin;
mod upgrade;

use core::iter;
use std::borrow::Cow;

use defuse_near_utils::UnwrapOrPanicError;
use defuse_nep245::receiver::MultiTokenReceiver;
use defuse_transfer_auth::{
    ext_transfer_auth,
    storage::{ContractStorage, StateInit as TransferAuthStateInit},
    TransferAuthContext,
};
use defuse_escrow_swap::ContractStorage as EscrowContractStorage;
use defuse_escrow_swap::ext_escrow;
use near_plugins::{access_control, access_control_any, AccessControllable, AccessControlRole};
use near_sdk::{
    env, ext_contract, json_types::U128, near, require, serde_json, AccountId, Gas, NearToken,
    PanicOnDefault, Promise, PromiseOrValue, PromiseResult,
    state_init::{StateInit, StateInitV1},
};

use crate::message::{EscrowParams, TransferMessage};
use crate::state::{ProxyConfig, RolesConfig};
use crate::{ext_mt_core, EscrowProxy, Role, RoleFlags};

#[access_control(role_type(Role))]
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    config: ProxyConfig,
}

impl Contract {
    fn derive_deteministic_escrow_per_fill_id(&self, msg_hash: [u8; 32]) -> StateInit {
        let state = TransferAuthStateInit {
            escrow_contract_id: self.config.escrow_swap_contract_id.clone(),
            auth_contract: self.config.auth_contract.clone(),
            on_auth_signer: self.config.auth_collee.clone(),
            authorizee: env::current_account_id(),
            msg_hash,
        };

        StateInit::V1(StateInitV1 {
            code: self.config.per_fill_contract_id.clone(),
            //TODO: get rid of unwrap
            data: ContractStorage::init_state(state).unwrap(),
        })
    }

    fn derive_deteministic_escrow_swap_id(&self, params: &EscrowParams) -> AccountId {
        let raw_state = EscrowContractStorage::init_state(params).unwrap();
        let state_init = StateInit::V1(StateInitV1 {
            code: self.config.escrow_swap_contract_id.clone(),
            data: raw_state,
        });
        state_init.derive_account_id()
    }
}

#[near]
impl Contract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(roles: RolesConfig, config: ProxyConfig) -> Contract {
        let mut contract = Self { config };
        contract.init_acl(roles);
        contract
    }

    fn init_acl(&mut self, roles: RolesConfig) {
        let mut acl = self.acl_get_or_init();
        require!(
            roles
                .super_admins
                .into_iter()
                .all(|super_admin| acl.add_super_admin_unchecked(&super_admin))
                && roles
                    .admins
                    .into_iter()
                    .flat_map(|(role, admins)| iter::repeat(role).zip(admins))
                    .all(|(role, admin)| acl.add_admin_unchecked(role, &admin))
                && roles
                    .grantees
                    .into_iter()
                    .flat_map(|(role, grantees)| iter::repeat(role).zip(grantees))
                    .all(|(role, grantee)| acl.grant_role_unchecked(role, &grantee)),
            "failed to set roles"
        );
    }
}

#[near]
impl EscrowProxy for Contract {
    fn config(&self) -> &ProxyConfig {
        &self.config
    }
}

#[near]
impl MultiTokenReceiver for Contract {
    #[allow(clippy::used_underscore_binding)]
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        //TODO: add helper method
        //TODO: use borsh here
        let context_hash = TransferAuthContext {
            sender_id: Cow::Borrowed(&sender_id),
            token_ids: Cow::Borrowed(&token_ids),
            amounts: Cow::Borrowed(&amounts),
            msg: Cow::Borrowed(&msg),
        }
        .hash();

        let auth_contract_state_init = self.derive_deteministic_escrow_per_fill_id(context_hash);
        let auth_contract_id = auth_contract_state_init.derive_account_id();
        let auth_call =
            Promise::new(auth_contract_id).state_init(auth_contract_state_init, NearToken::from_near(0));

        let _ = previous_owner_ids;

        let token_contract = env::predecessor_account_id();
        let transfer_message: TransferMessage = msg.parse().unwrap_or_panic_display();

        PromiseOrValue::Promise(
            ext_transfer_auth::ext_on(auth_call)
                .wait_for_authorization()
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(Gas::from_tgas(100))
                        .check_authorization_and_forward(
                            token_contract,
                            transfer_message.receiver_id,
                            token_ids,
                            amounts,
                            transfer_message.msg,
                        ),
                ),
        )
    }
}

#[near]
impl Contract {
    /// Callback after wait_for_authorization - checks result and forwards if authorized
    #[private]
    pub fn check_authorization_and_forward(
        &self,
        token_contract: AccountId,
        escrow_address: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        // Check the result of wait_for_authorization
        let is_authorized = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                serde_json::from_slice::<bool>(&value).unwrap_or(false)
            }
            PromiseResult::Failed => false,
        };

        if !is_authorized {
            near_sdk::env::panic_str("Authorization failed or timed out, refunding");
        }

        // Forward tokens to escrow
        PromiseOrValue::Promise(
            ext_mt_core::ext(token_contract)
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_static_gas(Gas::from_tgas(50))
                .mt_batch_transfer_call(
                    escrow_address,
                    token_ids,
                    amounts.clone(),
                    None, // approval
                    Some("proxy forward".to_string()), // memo
                    msg,
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(Gas::from_tgas(10))
                        .resolve_transfer(amounts),
                ),
        )
    }

    #[private]
    pub fn resolve_transfer(&self, original_amounts: Vec<U128>) -> Vec<U128> {
        match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                let transferred: Vec<U128> = serde_json::from_slice(&value).unwrap_or_else(|_| {
                    near_sdk::log!("Failed to parse escrow response, refunding all");
                    vec![U128(0); original_amounts.len()]
                });

                original_amounts
                    .iter()
                    .zip(transferred.iter())
                    .map(|(original, transferred)| U128(original.0.saturating_sub(transferred.0)))
                    .collect()
            }
            PromiseResult::Failed => {
                near_sdk::log!("Escrow transfer failed, refunding all");
                original_amounts
            }
        }
    }

    #[access_control_any(roles(Role::DAO))]
    pub fn cancel_escrow(&self, escrow_address: AccountId, params: EscrowParams) -> Promise {
        //TODO: adjust gas
        ext_escrow::ext(escrow_address)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(Gas::from_tgas(50))
            .escrow_close(params)
    }

    #[access_control_any(roles(Role::DAO, Role::Canceller))]
    pub fn authorize(&self, transfer_auth_id: AccountId) -> Promise {
        ext_transfer_auth::ext(transfer_auth_id)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(Gas::from_tgas(50))
            .authorize()
    }
}
