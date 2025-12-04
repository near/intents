use defuse_core::Salt;
use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountIdRef, NearToken, serde_json::json};

pub trait SaltManagerExt {
    async fn update_current_salt(&self, defuse_contract_id: &AccountIdRef) -> anyhow::Result<Salt>;

    async fn invalidate_salts(
        &self,
        defuse_contract_id: &AccountIdRef,
        salts: &[Salt],
    ) -> anyhow::Result<Salt>;
}

pub trait SaltManagerViewExt {
    async fn is_valid_salt(&self, salt: &Salt) -> anyhow::Result<bool>;

    async fn current_salt(&self) -> anyhow::Result<Salt>;
}

impl SaltManagerExt for SigningAccount {
    async fn update_current_salt(&self, defuse_contract_id: &AccountIdRef) -> anyhow::Result<Salt> {
        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("update_current_salt")
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn invalidate_salts(
        &self,
        defuse_contract_id: &AccountIdRef,
        salts: &[Salt],
    ) -> anyhow::Result<Salt> {
        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("invalidate_salts")
                    .json_args(&json!({ "salts": salts }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl SaltManagerViewExt for Account {
    async fn is_valid_salt(&self, salt: &Salt) -> anyhow::Result<bool> {
        self.call_view_function_json("is_valid_salt", json!({ "salt": salt }))
            .await
            .map_err(Into::into)
    }

    async fn current_salt(&self) -> anyhow::Result<Salt> {
        self.call_view_function_json("current_salt", ())
            .await
            .map_err(Into::into)
    }
}

impl SaltManagerViewExt for SigningAccount {
    async fn is_valid_salt(&self, salt: &Salt) -> anyhow::Result<bool> {
        self.account().is_valid_salt(salt).await
    }

    async fn current_salt(&self) -> anyhow::Result<Salt> {
        self.account().current_salt().await
    }
}
