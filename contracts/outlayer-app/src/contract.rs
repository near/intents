use defuse_borsh_utils::adapters::{AsWrap, Remainder};
use defuse_serde_utils::hex::AsHex;
use near_sdk::{AccountId, AccountIdRef, PanicOnDefault, assert_one_yocto, env, near, require};

use crate::{
    CodeLocation, Event, OutlayerApp, State,
    error::{ERR_NEW_CODE_HASH_MISMATCH, ERR_SELF_TRANSFER, ERR_UNAUTHORIZED},
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
    fn op_upload_code(&mut self, #[serializer(borsh)] code: AsWrap<Vec<u8>, Remainder>) {
        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );

        let code = code.into_inner();
        let code_hash = env::sha256_array(&code);

        require!(self.0.code_hash == code_hash, ERR_NEW_CODE_HASH_MISMATCH);

        self.store_code(code, code_hash);
    }

    #[payable]
    fn op_set_admin_id(&mut self, new_admin_id: AccountId) {
        assert_one_yocto();
        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        require!(!self.is_admin(&new_admin_id), ERR_SELF_TRANSFER);
        self.transfer_admin(new_admin_id);
    }

    #[payable]
    fn op_set_location(&mut self, location: CodeLocation) {
        assert_one_yocto();
        require!(
            self.is_admin(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        self.set_location(location);
    }

    fn op_admin_id(&self) -> &AccountId {
        &self.0.admin_id
    }

    fn op_code_hash(&self) -> AsHex<[u8; 32]> {
        self.0.code_hash.into()
    }

    fn op_code(&self) -> Option<crate::AsBase64<Vec<u8>>> {
        self.code().map(|b| crate::AsBase64(b.to_vec()))
    }

    fn op_location(&self) -> Option<CodeLocation> {
        self.0.location.clone()
    }
}

impl Contract {
    fn approve(&mut self, code_hash: [u8; 32]) {
        self.0.code_hash = code_hash;
        Event::Approve { code_hash }.emit();
    }

    fn code(&self) -> Option<&[u8]> {
        if !env::storage_has_key(State::CODE_PREFIX) {
            return None;
        }
        self.0.code.get().as_deref()
    }

    fn transfer_admin(&mut self, new_admin_id: AccountId) {
        Event::Transfer {
            old_admin_id: (&self.0.admin_id).into(),
            new_admin_id: (&new_admin_id).into(),
        }
        .emit();
        self.0.admin_id = new_admin_id;
    }

    fn store_code(&mut self, code: Vec<u8>, code_hash: [u8; 32]) {
        self.0.code.set(Some(code));
        Event::Upload { code_hash }.emit();

        self.set_location(CodeLocation::OnChain {
            account: env::current_account_id(),
            storage_prefix: State::CODE_PREFIX.to_vec(),
        });
    }

    fn set_location(&mut self, location: CodeLocation) {
        self.0.location = Some(location.clone());
        Event::SetLocation { location }.emit();
    }

    fn is_admin(&self, account_id: &AccountIdRef) -> bool {
        account_id == self.0.admin_id
    }
}
