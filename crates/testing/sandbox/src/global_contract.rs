use anyhow::Result;
use defuse_digest::{Digest, sha2::Sha256};
use near_kit::{
    AccountIdRef, NearToken, SigningNear,
    protocol::{Action, GlobalContractId, PublishMode},
    signer::SecretKey,
};

pub trait GlobalContract {
    async fn deploy_upgradable_global_contract(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractId>;

    async fn deploy_immutable_global_contract(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractId>;
}

impl GlobalContract for SigningNear {
    async fn deploy_upgradable_global_contract(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractId> {
        let secret_key = SecretKey::generate_ed25519();
        let account_id = target.as_ref();

        self.transaction(account_id)
            .add_action(Action::create_account())
            .add_action(Action::transfer(balance))
            .add_action(Action::add_full_access_key(secret_key.public_key()))
            .add_action(Action::publish(code, PublishMode::Updatable))
            .await?
            .result()?;

        Ok(GlobalContractId::AccountId(account_id.into()))
    }

    async fn deploy_immutable_global_contract(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractId> {
        let code = code.into();

        let id = GlobalContractId::CodeHash(Sha256::digest(&code).into());

        self.transaction(target.as_ref())
            .add_action(Action::create_account())
            .add_action(Action::transfer(balance))
            .add_action(Action::publish(code, PublishMode::Immutable))
            .await?
            .result()?;

        Ok(id)
    }
}
