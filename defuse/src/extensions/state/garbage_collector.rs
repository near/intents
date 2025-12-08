use defuse_core::Nonce;
use defuse_sandbox::{SigningAccount, anyhow, tx::FnCallBuilder};
use defuse_serde_utils::base64::AsBase64;
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json::json};

#[allow(async_fn_in_trait)]
pub trait GarbageCollectorExt {
    async fn cleanup_nonces(
        &self,
        defuse_contract_id: &AccountIdRef,
        data: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = Nonce>)>,
    ) -> anyhow::Result<()>;
}

impl GarbageCollectorExt for SigningAccount {
    async fn cleanup_nonces(
        &self,
        defuse_contract_id: &AccountIdRef,
        data: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = Nonce>)>,
    ) -> anyhow::Result<()> {
        let nonces = data
            .into_iter()
            .map(|(acc, nonces)| {
                let base64_nonces: Vec<AsBase64<Nonce>> =
                    nonces.into_iter().map(AsBase64).collect();
                (acc, base64_nonces)
            })
            .collect::<Vec<_>>();

        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("cleanup_nonces")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "nonces": nonces,
                    })),
            )
            .await?;

        Ok(())
    }
}
