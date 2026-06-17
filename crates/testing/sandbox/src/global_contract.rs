use anyhow::Result;
use near_kit::{Final, GlobalContractIdentifier, KeyPair, Near, NearToken, PublishMode};
use near_sdk::AccountIdRef;
use sha2::{Digest, Sha256};

pub trait GlobalContract {
    async fn deploy_upgradable_global_contract(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractIdentifier>;

    async fn deploy_immutable_global_contract(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractIdentifier>;
}

impl GlobalContract for Near {
    async fn deploy_upgradable_global_contract(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractIdentifier> {
        let kp = KeyPair::random();
        let account_id = target.as_ref();

        self.transaction(account_id)
            .create_account()
            .transfer(balance)
            .add_full_access_key(kp.public_key)
            .publish(code, PublishMode::Updatable)
            .wait_until(Final)
            .await?
            .result()?;

        Ok(GlobalContractIdentifier::AccountId(account_id.into()))
    }

    async fn deploy_immutable_global_contract(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractIdentifier> {
        let code = code.into();

        let hash: [u8; 32] = Sha256::digest(&code).into();
        let id = GlobalContractIdentifier::CodeHash(hash.into());

        self.transaction(target.as_ref())
            .create_account()
            .transfer(balance)
            .publish(code, PublishMode::Immutable)
            .wait_until(Final)
            .await?
            .result()?;

        Ok(id)
    }
}
