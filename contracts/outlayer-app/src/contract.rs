use defuse_serde_utils::hex::AsHex;
use near_sdk::{AccountId, AccountIdRef, PanicOnDefault, assert_one_yocto, env, near, require};

use crate::{
    Event, OutlayerApp, State, Url,
    error::{ERR_SELF_TRANSFER, ERR_UNAUTHORIZED},
};

#[near(
    contract_state(key = State::STATE_KEY),
    contract_metadata(
        standard(standard = "near-outlayer-app", version = "1.0.0")
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract(State);

#[near]
impl OutlayerApp for Contract {
    #[payable]
    fn op_approve(&mut self, new_hash: AsHex<[u8; 32]>) {
        assert_one_yocto();
        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        self.approve(new_hash.into_inner());
    }

    #[payable]
    fn op_set_admin_id(&mut self, new_admin_id: AccountId) {
        assert_one_yocto();
        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        require!(!self.is_admin(&new_admin_id), ERR_SELF_TRANSFER);
        self.set_admin(new_admin_id);
    }

    #[payable]
    fn op_set_code_uri(&mut self, url: Url) {
        assert_one_yocto();
        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        self.set_code_uri(url);
    }

    fn op_admin_id(&self) -> &AccountId {
        &self.0.admin_id
    }

    fn op_code_hash(&self) -> AsHex<[u8; 32]> {
        self.0.code_hash.into()
    }

    fn op_code_uri(&self) -> Url {
        self.0.code_url.clone()
    }
}

impl Contract {
    fn approve(&mut self, code_hash: [u8; 32]) {
        self.0.code_hash = code_hash;
        Event::Approve { code_hash }.emit();
    }

    fn set_admin(&mut self, new_admin_id: AccountId) {
        Event::Transfer {
            old_admin_id: (&self.0.admin_id).into(),
            new_admin_id: (&new_admin_id).into(),
        }
        .emit();
        self.0.admin_id = new_admin_id;
    }

    fn set_code_uri(&mut self, url: Url) {
        self.0.code_url = url.clone();
        Event::SetCodeUri { url }.emit();
    }

    fn is_admin(&self, account_id: &AccountIdRef) -> bool {
        account_id == self.0.admin_id
    }
}
