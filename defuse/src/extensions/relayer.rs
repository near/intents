use defuse_sandbox::{SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountIdRef, NearToken, PublicKey, serde_json::json};

#[allow(async_fn_in_trait)]
pub trait RelayerKeysExt {
    async fn add_relayer_key(
        &self,
        defuse_contract_id: &AccountIdRef,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;

    async fn delete_relayer_key(
        &self,
        defuse_contract_id: &AccountIdRef,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;
}

impl RelayerKeysExt for SigningAccount {
    async fn add_relayer_key(
        &self,
        defuse_contract_id: &AccountIdRef,
        public_key: &PublicKey,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("add_relayer_key")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(&json!({
                        "public_key": public_key,
                    })),
            )
            .await?;

        Ok(())
    }

    async fn delete_relayer_key(
        &self,
        defuse_contract_id: &AccountIdRef,
        public_key: &PublicKey,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("delete_relayer_key")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(&json!({
                        "public_key": public_key,
                    })),
            )
            .await?;

        Ok(())
    }
}
