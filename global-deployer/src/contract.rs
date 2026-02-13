use near_sdk::{AccountId, Gas, NearToken, Promise, assert_one_yocto, env, near, require};

use crate::{
    Contract, ContractExt, Event, GlobalDeployer,
    error::{ERR_SELF_TRANSFER, ERR_UNAUTHORIZED, ERR_WRONG_CODE_HASH},
};

const GD_AT_DEPLOY_GAS: Gas = Gas::from_tgas(15);

#[near]
impl GlobalDeployer for Contract {
    #[payable]
    fn gd_deploy(
        &mut self,
        #[serializer(borsh)] old_hash: [u8; 32],
        #[serializer(borsh)] new_code: Vec<u8>,
    ) -> Promise {
        require!(!env::attached_deposit().is_zero());
        self.require_owner();
        require!(self.0.code_hash == old_hash, ERR_WRONG_CODE_HASH);
        let new_code_hash = env::sha256_array(&new_code);

        // On receipt failure, refund goes to the receipt's predecessor — which for a
        // self-targeted promise is the contract itself. `.refund_to()` overrides this
        // so the deposit is refunded to the original caller instead. (NEP-616)
        Self::ext_on(
            Promise::new(env::current_account_id())
                .refund_to(env::refund_to_account_id())
                .transfer(env::attached_deposit())
                .deploy_global_contract_by_account_id(new_code),
        )
        .with_static_gas(GD_AT_DEPLOY_GAS)
        .with_unused_gas_weight(1)
        .gd_post_deploy(
            old_hash,
            new_code_hash,
            env::account_balance().saturating_sub(env::attached_deposit()),
        )
    }

    fn gd_owner_id(&self) -> AccountId {
        self.0.owner_id.clone()
    }

    fn gd_index(&self) -> u32 {
        self.0.index
    }

    fn gd_code_hash(&self) -> [u8; 32] {
        self.0.code_hash
    }

    #[payable]
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId) {
        assert_one_yocto();
        self.require_owner();
        require!(self.0.owner_id != receiver_id, ERR_SELF_TRANSFER);
        Event::Transfer {
            old_owner_id: self.0.owner_id.clone(),
            new_owner_id: receiver_id.clone(),
        }
        .emit();
        self.0.owner_id = receiver_id;
    }
}

#[near]
impl Contract {
    #[private]
    pub fn gd_post_deploy(
        &mut self,
        #[serializer(borsh)] old_hash: [u8; 32],
        #[serializer(borsh)] new_hash: [u8; 32],
        #[serializer(borsh)] initial_balance: NearToken,
    ) {
        require!(self.0.code_hash == old_hash, ERR_WRONG_CODE_HASH);
        self.0.code_hash = new_hash;
        Event::Deploy { old_hash, new_hash }.emit();

        let refund = env::account_balance().saturating_sub(initial_balance);
        if !refund.is_zero() {
            Promise::new(env::refund_to_account_id())
                .transfer(refund)
                .detach();
        }
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
