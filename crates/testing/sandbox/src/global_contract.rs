use anyhow::Result;
use near_kit::{Final, GlobalContractIdentifier, KeyPair, Near, PublishMode};
use near_sdk::{AccountId, NearToken, env::sha256_array};

pub trait GlobalContract {
    async fn deploy_upgradable_global_contract(
        &self,
        target: impl Into<AccountId>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractIdentifier>;

    async fn deploy_immutable_global_contract(
        &self,
        target: impl Into<AccountId>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractIdentifier>;
}

impl GlobalContract for Near {
    async fn deploy_upgradable_global_contract(
        &self,
        target: impl Into<AccountId>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractIdentifier> {
        let kp = KeyPair::random();
        let account_id = target.into();

        self.transaction(&account_id)
            .create_account()
            .transfer(balance)
            .add_full_access_key(kp.public_key)
            .publish(code, PublishMode::Updatable)
            .wait_until(Final)
            .await?;

        Ok(GlobalContractIdentifier::AccountId(account_id))
    }

    async fn deploy_immutable_global_contract(
        &self,
        target: impl Into<AccountId>,
        code: impl Into<Vec<u8>>,
        balance: NearToken,
    ) -> Result<GlobalContractIdentifier> {
        let code = code.into();

        let id = GlobalContractIdentifier::CodeHash(sha256_array(&code).into());

        self.transaction(target.into())
            .create_account()
            .transfer(balance)
            .publish(code, PublishMode::Immutable)
            .wait_until(Final)
            .await?;

        Ok(id)
    }
}
