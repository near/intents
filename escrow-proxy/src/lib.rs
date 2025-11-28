mod admin;
mod message;
mod upgrade;
mod storage;

use core::iter;
use std::collections::{HashMap, HashSet};

use defuse_crypto::{Curve, Ed25519, PublicKey, Signature};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use near_plugins::{AccessControlRole, access_control};
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, PromiseOrValue, PromiseResult, env,
    json_types::U128, near, require, serde_json,
};

pub use message::*;

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
    pub relay_public_key: PublicKey,
    pub nonces: HashSet<>,
}

#[near]
impl Contract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(relay_public_key: PublicKey, roles: RolesConfig) -> Contract {
        let mut contract = Self { relay_public_key };
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

    #[must_use]
    pub const fn get_relay_public_key(&self) -> &PublicKey {
        &self.relay_public_key
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
        let _ = previous_owner_ids;

        // 1. Parse message
        let transfer_msg: TransferMessage = match near_sdk::serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                near_sdk::log!("Parse error: {}", e);
                return PromiseOrValue::Value(amounts);
            }
        };

        // 2. Verify signature
        if !self.verify_signature(&transfer_msg) {
            near_sdk::log!("Invalid signature");
            return PromiseOrValue::Value(amounts);
        }

        // 3. Check deadline (nanoseconds since epoch)
        if u128::from(env::block_timestamp()) > transfer_msg.authorization.deadline.0 {
            near_sdk::log!("Deadline expired");
            return PromiseOrValue::Value(amounts);
        }

        // 4. Validate single token transfer
        if token_ids.len() != 1 || amounts.len() != 1 {
            near_sdk::log!("Only single token transfers supported");
            return PromiseOrValue::Value(amounts);
        }

        // 5. Validate amount matches
        if amounts[0] != transfer_msg.authorization.amount {
            near_sdk::log!("Amount mismatch");
            return PromiseOrValue::Value(amounts);
        }

        // TODO Phase 2: Verify token_ids[0] matches authorization.token
        // TODO Phase 2: Verify and commit nonce

        near_sdk::log!("Authorization valid for solver {}", sender_id);

        // 6. Forward tokens to escrow via mt_transfer_call
        let token_contract = env::predecessor_account_id();
        let escrow_address = transfer_msg.authorization.escrow.clone();

        // Build escrow message with fill parameters
        let escrow_msg = serde_json::to_string(&transfer_msg.escrow_params)
            .expect("escrow_params serialization should not fail");

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
                    escrow_msg,
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

impl Contract {
    fn verify_signature(&self, msg: &TransferMessage) -> bool {
        let PublicKey::Ed25519(relay_pk) = &self.relay_public_key else {
            return false;
        };
        let Signature::Ed25519(sig) = &msg.signature else {
            return false;
        };

        let hash = msg.authorization.hash();
        Ed25519::verify(sig, &hash, relay_pk).is_some()
    }
}
