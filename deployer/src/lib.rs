use std::collections::BTreeMap;

use near_sdk::serde_with::{hex::Hex, serde_as};
use near_sdk::{
    AccountId, CryptoHash, NearToken, PanicOnDefault, Promise, borsh, env, near, require,
};

pub const ERR_UNAUTHORIZED: &str = "unauthorized";
pub const ERR_SELF_TRANSFER: &str = "self-transfer";

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub owner_id: AccountId,
    pub index: u32,
}

impl State {
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
pub struct Contract(State);

pub const STATE_KEY: &[u8] = b"";

#[serde_as(crate = "near_sdk::serde_with")]
#[near(event_json(standard = "global-deployer"))]
pub enum Event {
    #[event_version("1.0.0")]
    Deploy(#[serde_as(as = "Hex")] CryptoHash),
}

#[near]
impl Contract {
    #[payable]
    pub fn gd_deploy(&mut self, #[serializer(borsh)] code: Vec<u8>) -> Promise {
        let deposit = env::attached_deposit();
        require!(!deposit.is_zero());
        self.require_owner();

        Event::Deploy(env::sha256_array(&code)).emit();

        Promise::new(env::current_account_id())
            .refund_to(env::refund_to_account_id())
            .transfer(deposit)
            .deploy_global_contract_by_account_id(code)
    }

    pub fn gd_owner_id(&self) -> AccountId {
        self.0.owner_id.clone()
    }

    pub fn gd_index(&self) -> u32 {
        self.0.index
    }

    #[payable]
    pub fn gd_transfer_ownership(&mut self, receiver_id: AccountId) {
        require!(env::attached_deposit() == NearToken::from_yoctonear(1));
        self.require_owner();
        require!(self.0.owner_id != receiver_id, ERR_SELF_TRANSFER);
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
