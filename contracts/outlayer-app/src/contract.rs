use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, NearToken, PanicOnDefault, assert_one_yocto, env, near, require,
};

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
#[repr(transparent)]
#[derive(PanicOnDefault)]
pub struct Contract(State);

#[near]
impl OutlayerApp for Contract {
    #[payable]
    fn oa_set_code(&mut self, new_hash: AsHex<[u8; 32]>, url: Url) {
        require!(
            env::attached_deposit() >= NearToken::from_yoctonear(1),
            "requires at least 1 yoctoNEAR"
        );
        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        self.set_code(new_hash.into_inner(), url);
    }

    #[payable]
    fn oa_transfer_admin(&mut self, new_admin_id: AccountId) {
        assert_one_yocto();
        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        require!(!self.is_admin(&new_admin_id), ERR_SELF_TRANSFER);
        self.transfer_admin(new_admin_id);
    }

    fn oa_admin_id(&self) -> &AccountId {
        &self.0.admin_id
    }

    fn oa_code_hash(&self) -> AsHex<[u8; 32]> {
        self.0.code_hash.into()
    }

    fn oa_code_uri(&self) -> Url {
        self.0.code_url.clone()
    }
}

impl Contract {
    fn set_code(&mut self, code_hash: [u8; 32], url: Url) {
        self.0.code_hash = code_hash;
        self.0.code_url = url.clone();
        Event::SetCodeHash { url, code_hash }.emit();
    }

    fn transfer_admin(&mut self, new_admin_id: AccountId) {
        Event::TransferAdmin {
            old_admin_id: (&self.0.admin_id).into(),
            new_admin_id: (&new_admin_id).into(),
        }
        .emit();
        self.0.admin_id = new_admin_id;
    }

    fn is_admin(&self, account_id: &AccountIdRef) -> bool {
        account_id == self.0.admin_id
    }
}
