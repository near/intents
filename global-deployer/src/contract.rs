use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, assert_one_yocto, env, near, require,
    state_init::{StateInit, StateInitV1},
};

use crate::{
    Event, GlobalDeployer, OwnerProxyState, State,
    error::{
        ERR_INSUFFICIENT_DEPOSIT, ERR_SAME_CODE, ERR_SELF_TRANSFER, ERR_UNAUTHORIZED,
        ERR_WRONG_CODE_HASH,
    },
};

const GD_AT_DEPLOY_GAS: Gas = Gas::from_tgas(15);

#[near(
    contract_state(key = State::STATE_KEY),
    contract_metadata(
        standard(standard = "global-deployer", version = "1.0.0")
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract(State);

#[near]
impl GlobalDeployer for Contract {
    #[payable]
    fn gd_deploy(
        &mut self,
        #[serializer(borsh)] old_hash: [u8; 32],
        #[serializer(borsh)] new_code: Vec<u8>,
    ) -> Promise {
        require!(!env::attached_deposit().is_zero(), ERR_INSUFFICIENT_DEPOSIT);

        let code_hash = env::sha256_array(&new_code);

        let predecessor = env::predecessor_account_id();
        if predecessor != self.0.owner_id {
            let derived_id = self.calculate_derived_account_id(code_hash);
            require!(predecessor == derived_id, ERR_UNAUTHORIZED);
        }

        require!(self.0.code_hash == old_hash, ERR_WRONG_CODE_HASH);
        let new_hash = env::sha256_array(&new_code);
        require!(new_hash != old_hash, ERR_SAME_CODE);

        let initial_balance = env::account_balance().saturating_sub(env::attached_deposit());

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
        .with_unused_gas_weight(1) // forward all remaining gas here
        .gd_post_deploy(old_hash.into(), new_hash.into(), initial_balance)
    }

    #[payable]
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId) {
        assert_one_yocto();
        self.require_owner();

        require!(self.0.owner_id != receiver_id, ERR_SELF_TRANSFER);
        Event::Transfer {
            old_owner_id: (&self.0.owner_id).into(),
            new_owner_id: (&receiver_id).into(),
        }
        .emit();
        self.0.owner_id = receiver_id;
    }

    fn gd_owner_id(&self) -> AccountId {
        self.0.owner_id.clone()
    }

    fn gd_index(&self) -> u32 {
        self.0.index
    }

    fn gd_code_hash(&self) -> AsHex<[u8; 32]> {
        self.0.code_hash.into()
    }
}

#[near]
impl Contract {
    #[private]
    pub fn gd_post_deploy(
        &mut self,
        old_hash: AsHex<[u8; 32]>,
        new_hash: AsHex<[u8; 32]>,
        initial_balance: NearToken,
    ) {
        let [old_hash, new_hash] = [old_hash, new_hash].map(AsHex::into_inner);

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

    fn calculate_derived_account_id(&self, new_hash: [u8; 32]) -> AccountId {
        let owner_contract_id = self
            .0
            .owner_contract_id
            .clone()
            .expect("owner_contract_id required");
        let proxy_state = OwnerProxyState::new(
            self.0.owner_id.clone(),
            self.0.code_hash,
            new_hash,
            env::current_account_id(),
        );
        StateInit::V1(StateInitV1 {
            code: owner_contract_id,
            data: proxy_state.state_init(),
        })
        .derive_account_id()
    }
}
