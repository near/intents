use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, Gas, NearToken, PanicOnDefault, Promise, assert_one_yocto, env, near,
    require,
};

use crate::{
    Event, GlobalDeployer, Reason, State,
    error::{ERR_NEW_CODE_HASH_MISMATCH, ERR_SELF_TRANSFER, ERR_UNAUTHORIZED, ERR_WRONG_CODE_HASH},
};

const GD_POST_DEPLOY_MIN_GAS: Gas = Gas::from_tgas(15);

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
    fn gd_approve(&mut self, old_hash: AsHex<[u8; 32]>, new_hash: AsHex<[u8; 32]>) {
        assert_one_yocto();
        require!(
            self.is_owner(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        require!(
            self.is_current_code_hash(&old_hash.into_inner()),
            ERR_WRONG_CODE_HASH
        );
        self.approve(
            new_hash.into_inner(),
            Reason::By(env::predecessor_account_id().into()),
        );
    }

    #[payable]
    fn gd_deploy(&mut self, #[serializer(borsh)] new_code: Vec<u8>) -> Promise {
        let new_hash = env::sha256_array(&new_code);
        require!(self.is_approved(&new_hash), ERR_NEW_CODE_HASH_MISMATCH);
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
        .with_static_gas(GD_POST_DEPLOY_MIN_GAS)
        .with_unused_gas_weight(1)
        .gd_post_deploy(new_hash.into(), initial_balance, env::attached_deposit())
    }

    #[payable]
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId) {
        assert_one_yocto();
        require!(
            self.is_owner(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        require!(!self.is_owner(&receiver_id), ERR_SELF_TRANSFER);
        self.transfer_ownership(receiver_id);
    }

    fn gd_owner_id(&self) -> AccountId {
        self.0.owner_id.clone()
    }

    fn gd_code_hash(&self) -> AsHex<[u8; 32]> {
        self.0.code_hash.into()
    }

    fn gd_approved_hash(&self) -> AsHex<[u8; 32]> {
        self.0.approved_hash.into()
    }
}

#[near]
impl Contract {
    #[private]
    pub fn gd_post_deploy(
        &mut self,
        new_hash: AsHex<[u8; 32]>,
        initial_balance: NearToken,
        deploy_deposit: NearToken,
    ) {
        let new_hash = new_hash.into_inner();
        require!(self.is_approved(&new_hash), ERR_NEW_CODE_HASH_MISMATCH);

        self.on_deploy(new_hash);

        let refund = env::account_balance()
            .saturating_sub(initial_balance)
            .min(deploy_deposit);
        if !refund.is_zero() {
            Promise::new(env::refund_to_account_id())
                .transfer(refund)
                .detach();
        }
    }
}

impl Contract {
    fn approve(&mut self, code_hash: [u8; 32], reason: Reason<'_>) {
        self.0.approved_hash = code_hash;
        Event::Approve { code_hash, reason }.emit();
    }

    fn on_deploy(&mut self, code_hash: [u8; 32]) {
        self.0.code_hash = code_hash;
        Event::Deploy { code_hash }.emit();
        self.approve(State::DEFAULT_HASH, Reason::Deploy(code_hash));
    }

    fn transfer_ownership(&mut self, new_owner_id: AccountId) {
        Event::Transfer {
            old_owner_id: (&self.0.owner_id).into(),
            new_owner_id: (&new_owner_id).into(),
        }
        .emit();

        self.0.owner_id = new_owner_id;
        self.approve(
            State::DEFAULT_HASH,
            Reason::By(self.0.owner_id.clone().into()),
        );
    }

    fn is_approved(&self, hash: &[u8; 32]) -> bool {
        self.0.approved_hash == *hash
    }

    fn is_current_code_hash(&self, hash: &[u8; 32]) -> bool {
        self.0.code_hash == *hash
    }

    fn is_owner(&self, account_id: &AccountIdRef) -> bool {
        account_id == self.0.owner_id
    }
}
