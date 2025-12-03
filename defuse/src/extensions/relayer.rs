use near_sdk::{AccountId, AccountIdRef, NearToken};
use serde_json::json;

pub trait RelayerKeysExt {
    async fn add_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;

    async fn delete_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;
}

impl RelayerKeysExt for Account {
    async fn add_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "add_relayer_key")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "public_key": public_key,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn delete_relayer_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "delete_relayer_key")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "public_key": public_key,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }
}
