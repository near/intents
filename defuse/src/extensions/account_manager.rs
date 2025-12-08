use defuse_core::{Nonce, crypto::PublicKey};
use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use defuse_serde_utils::base64::AsBase64;
use near_sdk::{AccountIdRef, Gas, NearToken, serde_json::json};

#[allow(async_fn_in_trait)]
pub trait AccountManagerExt: AccountViewExt {
    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountIdRef,
        public_key: PublicKey,
    ) -> anyhow::Result<()>;

    async fn remove_public_key(
        &self,
        defuse_contract_id: &AccountIdRef,
        public_key: PublicKey,
    ) -> anyhow::Result<()>;

    async fn disable_auth_by_predecessor_id(
        &self,
        defuse_contract_id: &AccountIdRef,
    ) -> anyhow::Result<()>;
}

#[allow(async_fn_in_trait)]
pub trait AccountViewExt {
    async fn has_public_key(
        &self,
        account_id: &AccountIdRef,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool>;

    async fn is_nonce_used(&self, account_id: &AccountIdRef, nonce: &Nonce)
    -> anyhow::Result<bool>;

    async fn is_auth_by_predecessor_id_enabled(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;
}

impl AccountManagerExt for SigningAccount {
    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountIdRef,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.into())
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
        defuse_contract_id: &AccountIdRef,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.into())
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
        defuse_contract_id: &AccountIdRef,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("disable_auth_by_predecessor_id")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .with_gas(Gas::from_tgas(10)),
            )
            .await?;
        Ok(())
    }
}

impl AccountViewExt for Account {
    async fn has_public_key(
        &self,
        account_id: &AccountIdRef,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.call_view_function_json(
            "has_public_key",
            json!({
                "account_id": account_id,
                "public_key": public_key,
            }),
        )
        .await
    }

    async fn is_nonce_used(
        &self,
        account_id: &AccountIdRef,
        nonce: &Nonce,
    ) -> anyhow::Result<bool> {
        self.call_view_function_json(
            "is_nonce_used",
            json!({
                "account_id": account_id,
                "nonce": AsBase64(nonce),
            }),
        )
        .await
    }

    async fn is_auth_by_predecessor_id_enabled(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.call_view_function_json(
            "is_auth_by_predecessor_id_enabled",
            json!({
                "account_id": account_id,
            }),
        )
        .await
    }
}

impl AccountViewExt for SigningAccount {
    async fn has_public_key(
        &self,
        account_id: &AccountIdRef,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.account().has_public_key(account_id, public_key).await
    }

    async fn is_nonce_used(
        &self,
        account_id: &AccountIdRef,
        nonce: &Nonce,
    ) -> anyhow::Result<bool> {
        self.account().is_nonce_used(account_id, nonce).await
    }

    async fn is_auth_by_predecessor_id_enabled(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.account()
            .is_auth_by_predecessor_id_enabled(account_id)
            .await
    }
}
