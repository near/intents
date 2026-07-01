use defuse_borsh_utils::{AsWrap, Remainder};
use defuse_digest::{Digest, sha2::Sha256};
use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, Gas, NearToken, PanicOnDefault, Promise, assert_one_yocto, env, near,
    require,
};

use crate::{
    Event, GlobalDeployer, Reason, State,
    error::{ERR_NEW_CODE_HASH_MISMATCH, ERR_SELF_TRANSFER, ERR_UNAUTHORIZED, ERR_WRONG_CODE_HASH},
};

#[near(
    contract_state(key = State::STATE_KEY),
    contract_metadata(
        standard(standard = "global-deployer", version = "1.0.0")
    )
)]
#[derive(PanicOnDefault)]
#[repr(transparent)]
pub struct Contract(State<'static>);

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
    fn gd_deploy(&mut self, #[serializer(borsh)] code: AsWrap<Vec<u8>, Remainder>) -> Promise {
        let code = code.into_inner();
        let code_hash = Sha256::digest(&code).into();

        require!(self.is_approved(&code_hash), ERR_NEW_CODE_HASH_MISMATCH);

        let initial_balance = env::account_balance().saturating_sub(env::attached_deposit());

        Self::ext_on(
            Promise::new(env::current_account_id())
                // 0. In case a receipt fails, re-direct the refund to the same
                // account which was specified for current receipt.
                .refund_to(env::refund_to_account_id())
                // 1. Transfer attached deposit to ourselves, so that it doesn't
                // affect our balance while in-flight. We could have attached
                // it to `gd_post_deploy()` below, but this balance is needed
                // for `deploy_global_contract_by_account_id` to succeed, so
                // we add a separate transfer action before.
                .transfer(env::attached_deposit())
                // 2. Deploy the global contract by our account_id
                .deploy_global_contract_by_account_id(code),
        )
        .with_static_gas(GD_POST_DEPLOY_MIN_GAS)
        .with_unused_gas_weight(1)
        // 3. Call post-deploy callback **in the same receipt**
        .gd_post_deploy(code_hash.into(), initial_balance, env::attached_deposit())
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
        self.0.owner_id.as_ref().to_owned()
    }

    fn gd_code_hash(&self) -> AsHex<[u8; 32]> {
        self.0.code_hash.into()
    }

    fn gd_approved_hash(&self) -> AsHex<[u8; 32]> {
        self.0.approved_hash.into()
    }
}

const GD_POST_DEPLOY_MIN_GAS: Gas = Gas::from_tgas(15);

#[near]
impl Contract {
    #[private]
    pub fn gd_post_deploy(
        &mut self,
        code_hash: AsHex<[u8; 32]>,
        initial_balance: NearToken,
        deploy_deposit: NearToken,
    ) {
        let code_hash = code_hash.into_inner();
        // check that approved hash hasn't changed while in-flight
        require!(self.is_approved(&code_hash), ERR_NEW_CODE_HASH_MISMATCH);

        self.on_deploy(code_hash);

        let refund = env::account_balance()
            .saturating_sub(initial_balance)
            .min(deploy_deposit);
        if !refund.is_zero() {
            // refund the rest to `refund_to` forwarded here by `gd_deploy()`
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

        // remove just-used approval
        self.approve(State::DEFAULT_HASH, Reason::Deploy(code_hash));
    }

    fn transfer_ownership(&mut self, new_owner_id: AccountId) {
        Event::Transfer {
            old_owner_id: self.0.owner_id.as_ref().into(),
            new_owner_id: (&new_owner_id).into(),
        }
        .emit();
        self.0.owner_id = new_owner_id.clone().into();

        // remove an approval from previous owner
        self.approve(
            State::DEFAULT_HASH,
            // pretend that new owner did it by himself,
            // since he would be interested in doing it anyway
            Reason::By(new_owner_id.into()),
        );
    }

    fn is_approved(&self, hash: &[u8; 32]) -> bool {
        self.0.approved_hash == *hash
    }

    fn is_current_code_hash(&self, hash: &[u8; 32]) -> bool {
        self.0.code_hash == *hash
    }

    fn is_owner(&self, account_id: &AccountIdRef) -> bool {
        *self.0.owner_id == *account_id
    }
}
