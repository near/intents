use defuse::core::Nonce;
use defuse_serde_utils::base64::AsBase64;
use near_sdk::{AccountId, NearToken};
use serde_json::json;

use crate::utils::test_log::TestLog;

pub trait GarbageCollectorExt {
    async fn cleanup_nonces(
        &self,
        defuse_contract_id: &AccountId,
        data: &[(AccountId, Vec<Nonce>)],
    ) -> anyhow::Result<TestLog>;
}

impl GarbageCollectorExt for near_workspaces::Account {
    async fn cleanup_nonces(
        &self,
        defuse_contract_id: &AccountId,
        data: &[(AccountId, Vec<Nonce>)],
    ) -> anyhow::Result<TestLog> {
        let nonces = data
            .iter()
            .map(|(acc, nonces)| {
                let base64_nonces: Vec<AsBase64<Nonce>> =
                    nonces.iter().map(|nonce| AsBase64(*nonce)).collect();
                (acc.clone(), base64_nonces)
            })
            .collect::<Vec<(AccountId, Vec<AsBase64<Nonce>>)>>();

        let res = self
            .call(defuse_contract_id, "cleanup_nonces")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "nonces": nonces,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()
            .map(TestLog::from)?;

        Ok(res)
    }
}

impl GarbageCollectorExt for near_workspaces::Contract {
    async fn cleanup_nonces(
        &self,
        defuse_contract_id: &AccountId,
        data: &[(AccountId, Vec<Nonce>)],
    ) -> anyhow::Result<TestLog> {
        self.as_account()
            .cleanup_nonces(defuse_contract_id, data)
            .await
    }
}
