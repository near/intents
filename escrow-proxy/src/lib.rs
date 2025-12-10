mod admin;
#[cfg(feature = "test-utils")]
pub mod ext;
mod message;
mod storage;
mod upgrade;

use core::iter;
use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::{
    env::keccak256,
    state_init::{StateInit, StateInitV1}, Promise,
};
use std::{borrow::Cow, collections::{BTreeMap, HashMap, HashSet}};

use near_plugins::{AccessControllable, access_control_any};
use defuse_auth_call::{ext_auth_callee, AuthCallee};
use defuse_crypto::{Curve, Ed25519, PublicKey, Signature};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use near_plugins::{AccessControlRole, access_control};
use near_sdk::{
    AccountId, Gas, GlobalContractId, NearToken, PanicOnDefault, PromiseOrValue, PromiseResult,
    env, ext_contract, json_types::U128, near, require, serde_json,
};

use defuse_transfer_auth::{ext_transfer_auth, storage::{ContractStorage, StateInit as TransferAuthStateInit}, TransferAuthContext};
use defuse_escrow_swap::ContractStorage as EscrowContractStorage;
use defuse_escrow_swap::action::TransferMessage as EscrowTransferMessage;

#[near(serializers = [json])]
#[derive(AccessControlRole, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Can upgrade the contract
    DAO,
    /// Can upgrade the contract
    Upgrader,
    /// Can call cancel on the proxy contracts
    Canceller,
}

pub use message::*;

#[ext_contract(ext_escrow_proxy)]
pub trait EscrowProxy {
    fn config(&self) -> &ProxyConfig;
}

/// Configuration for role-based access control
#[near(serializers = [json])]
#[derive(Debug, Clone, Default)]
pub struct RolesConfig {
    pub super_admins: HashSet<AccountId>,
    pub admins: HashMap<Role, HashSet<AccountId>>,
    pub grantees: HashMap<Role, HashSet<AccountId>>,
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    pub per_fill_contract_id: GlobalContractId,
    pub escrow_swap_contract_id: GlobalContractId,
    pub auth_contract: AccountId,
    pub auth_collee: AccountId,
}

#[access_control(role_type(Role))]
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    config: ProxyConfig,
}

impl Contract {
    fn derive_deteministic_escrow_per_fill_id(
        &self,
        msg_hash: [u8; 32],
    ) -> StateInit{

        let state = TransferAuthStateInit {
            escrow_contract_id: self.config.escrow_swap_contract_id.clone() ,
            auth_contract: self.config.auth_contract.clone(),
            on_auth_signer: self.config.auth_collee.clone(),
            authorizee: env::current_account_id(),
            msg_hash,
        };

        let state_init = StateInit::V1(StateInitV1 {
            code: self.config.per_fill_contract_id.clone(),
            //TODO: get rid of unwrap
            data: ContractStorage::init_state(state).unwrap(),
        });
        state_init

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
    pub fn new(
        roles: RolesConfig,
        config: ProxyConfig
    ) -> Contract {
        let mut contract = Self {
            config
        };
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
        }.hash();

        let auth_contract_state_init = self.derive_deteministic_escrow_per_fill_id(context_hash);
        let auth_contract_id = auth_contract_state_init.derive_account_id();
        let mut auth_call = Promise::new(auth_contract_id).state_init(auth_contract_state_init, NearToken::from_near(0));

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

    /// Callback to resolve the escrow transfer result
    #[private]
    pub fn resolve_transfer(&self, original_amounts: Vec<U128>) -> Vec<U128> {
        match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                //TODO:: aling the outputs its not mt_transfer_call

                // mt_transfer_call returns the refunded amounts
                // We pass through whatever the escrow refunded
                serde_json::from_slice::<Vec<U128>>(&value).unwrap_or_else(|_| {
                    near_sdk::log!("Failed to parse escrow response");
                    original_amounts
                })
            }
            PromiseResult::Failed => {
                near_sdk::log!("Escrow transfer failed, refunding");
                original_amounts
            }
        }
    }

    //TODO: implement as trait imported from defuse
    #[access_control_any(roles(Role::DAO))]
    pub fn cancel_escrow(&self, escrow_address: AccountId) {
        // ext_mt_core::ext(escrow_address)
        //     .with_attached_deposit(NearToken::from_yoctonear(1))
        //     .with_static_gas(Gas::from_tgas(50))
        //     .cancel_escrow();
    }


    //TODO: implement as trait imported from defuse
    // just pass globla contract id
    #[access_control_any(roles(Role::DAO, Role::Canceller))]
    pub fn close_auth(&self, transfer_auth_id: AccountId) {
        // let hash: [u8; 32] = keccak256(msg.as_bytes()).try_into().unwrap();
        // let auth_contract_id = self.derive_deteministic_escrow_per_fill_id(solver_id, hash);
        //
        // ext_transfer_auth::ext(auth_contract_id.derive_account_id())
        //     .with_attached_deposit(NearToken::from_yoctonear(1))
        //     .with_static_gas(Gas::from_tgas(50))
        //     .close();
    }
}

// fix JsonSchema macro bug
#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
use near_sdk::serde;
