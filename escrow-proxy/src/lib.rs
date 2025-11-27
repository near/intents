mod message;

use defuse_crypto::{Curve, Ed25519, PublicKey, Signature};
use defuse_nep245::receiver::MultiTokenReceiver;
use near_sdk::{env, json_types::U128, near, AccountId, PanicOnDefault, PromiseOrValue};

pub use message::*;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub relay_public_key: PublicKey,
    pub owner_id: AccountId,
}

#[near]
impl Contract {
    #[init]
    pub fn new(relay_public_key: PublicKey, owner_id: AccountId) -> Self {
        Self {
            relay_public_key,
            owner_id,
        }
    }

    pub fn get_relay_public_key(&self) -> &PublicKey {
        &self.relay_public_key
    }

    pub fn get_owner(&self) -> &AccountId {
        &self.owner_id
    }
}

#[near]
impl MultiTokenReceiver for Contract {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        _previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
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
        // TODO Phase 2: Forward to escrow via mt_transfer_call

        near_sdk::log!("Authorization valid for solver {}", sender_id);

        // Phase 1: Return full refund (validation only)
        PromiseOrValue::Value(amounts)
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
