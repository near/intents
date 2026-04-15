use defuse_serde_utils::hex::AsHex;
use near_sdk::{AccountId, AccountIdRef, NearToken, PanicOnDefault, env, near, require};

use crate::{
    Event, OutlayerApp, State,
    error::{
        ERR_INSUFFICIENT_DEPOSIT, ERR_REQUIRE_ONE_YOCTO, ERR_SELF_TRANSFER, ERR_UNAUTHORIZED,
        ERR_WRONG_CODE_HASH,
    },
};

#[near(
    contract_state(key = State::STATE_KEY),
    contract_metadata(
        standard(standard = "outlayer-app", version = "1.0.0")
    )
)]
#[derive(PanicOnDefault)]
#[repr(transparent)]
pub struct Contract(State);

#[near]
impl OutlayerApp for Contract {
    #[payable]
    fn oa_set_code(
        &mut self,
        old_code_hash: AsHex<[u8; 32]>,
        new_code_hash: AsHex<[u8; 32]>,
        new_code_url: String,
    ) {
        require!(
            env::attached_deposit() >= NearToken::from_yoctonear(1),
            ERR_INSUFFICIENT_DEPOSIT
        );
        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        require!(
            self.is_current_code_hash(&old_code_hash.into_inner()),
            ERR_WRONG_CODE_HASH
        );
        self.set_code(new_code_hash.into_inner(), new_code_url);
    }

    #[payable]
    fn oa_transfer_admin(&mut self, new_admin_id: AccountId) {
        require!(
            env::attached_deposit() == NearToken::from_yoctonear(1),
            ERR_REQUIRE_ONE_YOCTO
        );

        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        require!(!self.is_admin(&new_admin_id), ERR_SELF_TRANSFER);
        self.transfer_admin(new_admin_id);
    }

    fn oa_admin_id(&self) -> AccountId {
        self.0.admin_id.clone()
    }

    fn oa_code_hash(&self) -> AsHex<[u8; 32]> {
        self.0.code_hash.into()
    }

    fn oa_code_url(&self) -> String {
        self.0.code_url.clone()
    }
}

impl Contract {
    fn set_code(&mut self, code_hash: [u8; 32], url: String) {
        self.0.code_hash = code_hash;
        self.0.code_url = url.clone();
        Event::SetCode {
            hash: code_hash,
            url,
        }
        .emit();
    }

    fn transfer_admin(&mut self, new_admin_id: AccountId) {
        Event::TransferAdmin {
            old_admin_id: (&self.0.admin_id).into(),
            new_admin_id: (&new_admin_id).into(),
        }
        .emit();
        self.0.admin_id = new_admin_id;
    }

    fn is_current_code_hash(&self, hash: &[u8; 32]) -> bool {
        self.0.code_hash == *hash
    }

    fn is_admin(&self, account_id: &AccountIdRef) -> bool {
        account_id == self.0.admin_id
    }
}
