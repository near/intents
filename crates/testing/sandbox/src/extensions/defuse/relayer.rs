use crate::{SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountId, NearToken, PublicKey, serde_json::json};

pub trait RelayerKeysExt {
    async fn add_relayer_key(
        &self,
        contract_id: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;

    async fn delete_relayer_key(
        &self,
        contract_id: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;
}

impl RelayerKeysExt for SigningAccount {
    async fn add_relayer_key(
        &self,
        contract_id: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> anyhow::Result<()> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("add_relayer_key")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "public_key": public_key,
                    })),
            )
            .await?;

        Ok(())
    }

    async fn delete_relayer_key(
        &self,
        contract_id: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> anyhow::Result<()> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("delete_relayer_key")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "public_key": public_key,
                    })),
            )
            .await?;

        Ok(())
    }
}
