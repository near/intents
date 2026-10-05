use anyhow::Result;
use defuse_digest::{Digest, sha2::Sha256};
use near_kit::{
    Near, NearToken,
    protocol::{AccountIdRef, GlobalContractId, PublishMode},
    signer::SecretKey,
    transaction::Final,
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

impl GlobalContract for Near {
    async fn deploy_upgradable_global_contract(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractId> {
        let secret_key = SecretKey::generate_ed25519();
        let account_id = target.as_ref();

        self.transaction(account_id)
            .create_account()
            .transfer(balance)
            .add_full_access_key(secret_key.public_key())
            .publish(code, PublishMode::Updatable)
            .wait_until::<Final>()
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
            .create_account()
            .transfer(balance)
            .publish(code, PublishMode::Immutable)
            .wait_until::<Final>()
            .await?
            .result()?;

        Ok(id)
    }
}
