use std::collections::BTreeMap;

use near_sdk::{
    AccountId, CryptoHash, NearToken, PanicOnDefault, Promise, borsh, env, near, require,
};
use serde_with::{hex::Hex, serde_as};

pub const ERR_UNAUTHORIZED: &str = "unauthorized";

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    pub owner_id: AccountId,
    pub index: u32,
}

impl Storage {
    pub fn state_init(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            STATE_KEY.to_vec(),
            borsh::to_vec(&self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}

#[near(contract_state(key = STATE_KEY), contract_metadata(standard(standard = "global-deployer", version = "1.0.0")))]
#[derive(PanicOnDefault)]
pub struct Contract(Storage);

pub const STATE_KEY: &[u8] = b"";

#[serde_as]
#[near(event_json(standard = "controller"))]
pub enum Event {
    #[event_version("1.0.0")]
    Deploy(#[serde_as(as = "Hex")] CryptoHash),
}

#[near]
impl Contract {
    #[payable]
    pub fn gd_deploy(&mut self, #[serializer(borsh)] code: Vec<u8>) -> Promise {
        let deposit = env::attached_deposit();
        require!(deposit >= NearToken::from_yoctonear(1));
        self.require_owner();

        let hash = env::sha256_array(&code);

        Event::Deploy(hash).emit();

        Promise::new(env::current_account_id())
            .refund_to(env::refund_to_account_id())
            .transfer(deposit)
            .deploy_global_contract_by_account_id(code)
    }

    #[cfg(feature = "use-me")]
    #[payable]
    pub fn gd_use_me(&mut self) -> Promise {
        self.require_owner();
        Promise::new(env::current_account_id())
            .use_global_contract_by_account_id(env::current_account_id())
    }

    pub fn gd_state(&self) -> Storage {
        self.0.clone()
    }

    #[payable]
    pub fn gd_transfer_ownership(&mut self, receiver_id: AccountId) {
        require!(env::attached_deposit() == NearToken::from_yoctonear(1));
        self.require_owner();
        require!(self.0.owner_id != receiver_id, "same");
        self.0.owner_id = receiver_id;
    }
}

impl Contract {
    fn require_owner(&self) {
        require!(
            env::predecessor_account_id() == self.0.owner_id,
            ERR_UNAUTHORIZED
        );
    }
}
