use defuse::core::{Nonce, crypto::PublicKey};
use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use defuse_serde_utils::base64::AsBase64;
use near_sdk::{AccountId, AccountIdRef, Gas, NearToken, serde_json::json};

pub trait AccountManagerExt {
    async fn add_public_key(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;

    async fn remove_public_key(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> anyhow::Result<()>;

    async fn disable_auth_by_predecessor_id(
        &self,
        defuse_contract_id: impl Into<AccountId>,
    ) -> anyhow::Result<()>;
}

impl AccountManagerExt for SigningAccount {
    async fn add_public_key(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("add_public_key")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "public_key": public_key,
                    })),
            )
            .await?;

        Ok(())
    }

    async fn remove_public_key(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("remove_public_key")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "public_key": public_key,
                    })),
            )
            .await?;

        Ok(())
    }

    async fn disable_auth_by_predecessor_id(
        &self,
        defuse_contract_id: impl Into<AccountId>,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("disable_auth_by_predecessor_id")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .with_gas(Gas::from_tgas(10)),
            )
            .await?;

        Ok(())
    }
}

pub trait AccountViewExt {
    async fn has_public_key(
        &self,
        account_id: impl AsRef<AccountIdRef>,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool>;

    async fn is_nonce_used(
        &self,
        account_id: impl AsRef<AccountIdRef>,
        nonce: &Nonce,
    ) -> anyhow::Result<bool>;

    async fn is_auth_by_predecessor_id_enabled(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<bool>;
}

impl AccountViewExt for Account {
    async fn has_public_key(
        &self,
        account_id: impl AsRef<AccountIdRef>,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.call_view_function_json(
            "has_public_key",
            json!({
                "account_id": account_id.as_ref(),
                "public_key": public_key,
            }),
        )
        .await
    }

    async fn is_nonce_used(
        &self,
        account_id: impl AsRef<AccountIdRef>,
        nonce: &Nonce,
    ) -> anyhow::Result<bool> {
        self.call_view_function_json(
            "is_nonce_used",
            json!({
                "account_id": account_id.as_ref(),
                "nonce": AsBase64(nonce),
            }),
        )
        .await
    }

    async fn is_auth_by_predecessor_id_enabled(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<bool> {
        self.call_view_function_json(
            "is_auth_by_predecessor_id_enabled",
            json!({
                "account_id": account_id.as_ref(),
            }),
        )
        .await
    }
}
