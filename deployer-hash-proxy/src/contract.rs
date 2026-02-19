use defuse_global_deployer::ext_global_deployer;
use near_sdk::{PanicOnDefault, Promise, env, near, require};

use crate::{
    DeployerProxyHash, Event, ExtraParams, State,
    error::{ERR_MISSING_APPROVAL, ERR_UNAUTHORIZED, ERR_WRONG_CODE_HASH},
};

#[near(
    contract_state(key = State::STATE_KEY),
    contract_metadata(
        standard(standard = "global-deployer", version = "1.0.0")
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract(State);

#[near]
impl DeployerProxyHash for Contract {
    #[payable]
    fn hp_approve(&mut self) {
        self.require_owner();
        Event::Approved {
            old_hash: self.0.proxy.old_hash,
            new_hash: self.0.proxy.new_hash,
        }
        .emit();
        let mut extra = ExtraParams::read();
        extra.approved = true;
        extra.write();
    }

    #[payable]
    fn hp_exec(&mut self, #[serializer(borsh)] code: Vec<u8>) -> Promise {
        require!(ExtraParams::read().approved, ERR_MISSING_APPROVAL);

        let code_hash = env::sha256_array(&code);
        require!(self.0.proxy.new_hash == code_hash, ERR_WRONG_CODE_HASH);

        ext_global_deployer::ext(self.0.proxy.deployer_instance.clone())
            .with_attached_deposit(env::attached_deposit())
            .gd_deploy(self.0.proxy.old_hash, code)
    }
}

impl Contract {
    fn require_owner(&self) {
        require!(
            env::predecessor_account_id() == self.0.proxy.owner_id,
            ERR_UNAUTHORIZED
        );
    }
}
