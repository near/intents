use defuse_borsh_utils::adapters::{AsWrap, Remainder};
use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, AccountIdRef, PanicOnDefault, assert_one_yocto, env, near, require,
};

use crate::{
    Event, OutlayerProject, State, WasmLocation,
    error::{
        ERR_NEW_CODE_HASH_MISMATCH, ERR_SELF_TRANSFER, ERR_UNAUTHORIZED,
    },
};

#[near(
    contract_state(key = State::STATE_KEY),
    contract_metadata(
        standard(standard = "near-outlayer-project", version = "1.0.0")
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract(State);

#[near]
impl OutlayerProject for Contract {
    #[payable]
    fn oc_approve(&mut self, new_hash: AsHex<[u8; 32]>) {
        assert_one_yocto();
        require!(
            self.is_updater(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        self.approve(new_hash.into_inner());
    }

    #[payable]
    fn oc_upload_wasm(&mut self, #[serializer(borsh)] wasm: AsWrap<Vec<u8>, Remainder>) {
        require!(
            self.is_updater(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );

        let wasm = wasm.into_inner();
        let wasm_hash = env::sha256_array(&wasm);

        require!(self.0.wasm_hash == wasm_hash, ERR_NEW_CODE_HASH_MISMATCH);

        self.0.wasm.set(Some(wasm));
        self.0.location = Some(WasmLocation::OnChain {
            account: env::current_account_id(),
            storage_prefix: State::WASM_PREFIX.to_vec(),
        });

        Event::Upload { code_hash: wasm_hash }.emit();
    }

    #[payable]
    fn oc_set_updater_id(&mut self, new_updater_id: AccountId) {
        assert_one_yocto();
        require!(
            self.is_updater(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        require!(!self.is_updater(&new_updater_id), ERR_SELF_TRANSFER);


        Event::Transfer {
            old_updater_id: (&self.0.updater_id).into(),
            new_updater_id: (&new_updater_id).into(),
        }
        .emit();

        self.0.updater_id = new_updater_id;
        self.approve(State::DEFAULT_HASH);
    }

    #[payable]
    fn oc_set_location(&mut self, location: WasmLocation) {
        assert_one_yocto();
        require!(
            self.is_updater(&env::predecessor_account_id()),
            ERR_UNAUTHORIZED
        );
        self.0.location = Some(location.clone());
        Event::SetLocation { location }.emit();
    }

    fn oc_updater_id(&self) -> &AccountId {
        &self.0.updater_id
    }

    fn oc_wasm_hash(&self) -> AsHex<[u8; 32]> {
        self.0.wasm_hash.into()
    }

    fn oc_wasm(&self) -> Option<crate::AsBase64<Vec<u8>>> {
        self.wasm().map(|b| crate::AsBase64(b.to_vec()))
    }

    fn oc_location(&self) -> Option<WasmLocation> {
        self.0.location.clone()
    }
}

impl Contract {
    fn approve(&mut self, code_hash: [u8; 32]) {
        self.0.wasm_hash = code_hash;
        Event::Approve { code_hash }.emit();
    }

    fn wasm(&self) -> Option<&[u8]> {
        if !env::storage_has_key(State::WASM_PREFIX) {
            return None;
        }
        self.0.wasm.get().as_deref()
    }

    fn is_updater(&self, account_id: &AccountIdRef) -> bool {
        account_id == self.0.updater_id
    }
}
