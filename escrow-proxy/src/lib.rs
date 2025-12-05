mod admin;
mod escrow_params;
mod message;
mod storage;
mod upgrade;

use core::iter;
use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::{
    env::keccak256,
    state_init::{StateInit, StateInitV1},
};
use std::collections::{HashMap, HashSet};

use defuse_auth_call::AuthCallee;
use defuse_crypto::{Curve, Ed25519, PublicKey, Signature};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use near_plugins::{AccessControlRole, access_control};
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, PromiseOrValue, PromiseResult, env,
    json_types::U128, near, require, serde_json,
};

use defuse_transfer_auth::storage::{ContractStorage, State};

#[near(serializers = [json])]
#[derive(AccessControlRole, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Can upgrade the contract
    Owner,
    /// Can call cancel on the proxy contract (forwarded to escrow)
    Canceller,
    /// Can rotate the relay public key
    KeyManager,
}

pub use message::*;

/// Configuration for role-based access control
#[near(serializers = [json])]
#[derive(Debug, Clone, Default)]
pub struct RolesConfig {
    pub super_admins: HashSet<AccountId>,
    pub admins: HashMap<Role, HashSet<AccountId>>,
    pub grantees: HashMap<Role, HashSet<AccountId>>,
}

#[access_control(role_type(Role))]
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    // pub relay_public_key: PublicKey,
    pub per_fill_global_contract_id: AccountId,
    pub escrow_swap_global_contract_id: AccountId,
}

impl Contract {
    fn derive_deteministic_escrow_per_fill_id(
        &self,
        solver_id: AccountId,
        msg_hash: [u8; 32],
    ) -> AccountId {
        let state = State {
            solver_id,
            escrow_contract_id: todo!(),
            auth_contract: todo!(),
            auth_callee: todo!(),
            querier: todo!(),
            msg_hash,
        };

        let state_init = StateInit::V1(StateInitV1 {
            code: near_sdk::GlobalContractId::AccountId(self.per_fill_global_contract_id.clone()),
            //TODO: get rid of unwrap
            data: ContractStorage::init_state(state).unwrap(),
        });

        state_init.derive_account_id()
    }

    fn derive_deteministic_escrow_swap_id(&self) -> AccountId {
        unimplemented!()
    }
}

#[near]
impl Contract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(
        roles: RolesConfig,
        per_fill_global_contract_id: AccountId,
        escrow_swap_global_contract_id: AccountId,
    ) -> Contract {
        let mut contract = Self {
            per_fill_global_contract_id,
            escrow_swap_global_contract_id,
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
        //NOTE: instead of serializing and desrializing just hash origin message, relay
        //can do the same when creating Auth intent with state init
        //NOTE: its unique because of salt
        let hash: [u8; 32] = keccak256(msg.as_bytes()).try_into().unwrap();
        let auth_contract_id = self.derive_deteministic_escrow_per_fill_id(sender_id, hash);

        let _ = previous_owner_ids;

        // 4. Validate single token transfer
        if token_ids.len() != 1 || amounts.len() != 1 {
            near_sdk::log!("Only single token transfers supported");
            return PromiseOrValue::Value(amounts);
        }

        // if token_ids[0] != transfer_msg.authorization.token {
        //     near_sdk::log!("Token mismatch");
        //     return PromiseOrValue::Value(amounts);
        // }
        //
        // TODO Phase 2: Verify token_ids[0] matches authorization.token
        // TODO Phase 2: Verify and commit nonce

        // 6. Forward tokens to escrow via mt_transfer_call
        let token_contract = env::predecessor_account_id();
        let escrow_address = self.derive_deteministic_escrow_swap_id();

        // // Build escrow message with fill parameters
        // let escrow_msg = serde_json::to_string(&transfer_msg.escrow_params)
        //     .expect("escrow_params serialization should not fail");

        // Call mt_transfer_call on the token contract to forward to escrow
        PromiseOrValue::Promise(
            ext_mt_core::ext(token_contract)
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_static_gas(Gas::from_tgas(50))
                .mt_transfer_call(
                    escrow_address,
                    token_ids[0].clone(),
                    amounts[0],
                    None, // approval
                    None, // memo
                    //TODO: will the hashes still be fine when serializing and desrializzint??
                    //NOTE: for now lets pass origin message nad maybe have it aligned with
                    //escrow-swap so that it ignores extra parameters, so we dont have to serialize
                    //& ddeserialize
                    msg,
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(Gas::from_tgas(10))
                        .resolve_transfer(amounts),
                ),
        )
    }
}

#[near]
impl Contract {
    /// Callback to resolve the escrow transfer result
    #[private]
    pub fn resolve_transfer(&self, original_amounts: Vec<U128>) -> Vec<U128> {
        match env::promise_result(0) {
            PromiseResult::Successful(value) => {
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
}

// fix JsonSchema macro bug
#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
use near_sdk::serde;
