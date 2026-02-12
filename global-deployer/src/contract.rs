use near_sdk::{AccountId, NearToken, Promise, env, near, require};

use crate::{Contract, ContractExt, Event, GlobalDeployer};

#[near]
impl GlobalDeployer for Contract {
    #[payable]
    fn gd_deploy(&mut self, #[serializer(borsh)] code: Vec<u8>) -> Promise {
        let deposit = env::attached_deposit();
        require!(!deposit.is_zero());
        self.require_owner();

        Event::Deploy(env::sha256_array(&code)).emit();

        Promise::new(env::current_account_id())
            .refund_to(env::refund_to_account_id())
            .transfer(deposit)
            .deploy_global_contract_by_account_id(code)
    }

    fn gd_owner_id(&self) -> AccountId {
        self.0.owner_id.clone()
    }

    fn gd_index(&self) -> u32 {
        self.0.index
    }

    #[payable]
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId) {
        require!(env::attached_deposit() == NearToken::from_yoctonear(1));
        self.require_owner();
        require!(
            self.0.owner_id != receiver_id,
            crate::error::ERR_SELF_TRANSFER
        );
        self.0.owner_id = receiver_id;
    }
}

impl Contract {
    fn require_owner(&self) {
        require!(
            env::predecessor_account_id() == self.0.owner_id,
            crate::error::ERR_UNAUTHORIZED
        );
    }
}
