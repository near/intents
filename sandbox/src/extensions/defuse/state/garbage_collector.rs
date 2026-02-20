use crate::{SigningAccount, anyhow, tx::FnCallBuilder};
use defuse::core::Nonce;
use defuse_serde_utils::base64::AsBase64;
use near_api::types::transaction::result::ExecutionSuccess;
use near_sdk::{AccountId, NearToken, serde_json::json};

pub trait GarbageCollectorExt {
    async fn cleanup_nonces(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        data: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = Nonce>)>,
    ) -> anyhow::Result<ExecutionSuccess>;
}

impl GarbageCollectorExt for SigningAccount {
    async fn cleanup_nonces(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        data: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = Nonce>)>,
    ) -> anyhow::Result<ExecutionSuccess> {
        let nonces = data
            .into_iter()
            .map(|(acc, nonces)| {
                let base64_nonces: Vec<AsBase64<Nonce>> =
                    nonces.into_iter().map(AsBase64).collect();
                (acc, base64_nonces)
            })
            .collect::<Vec<_>>();

        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("cleanup_nonces")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "nonces": nonces,
                    })),
            )
            .await
    }
}
