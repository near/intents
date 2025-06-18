use defuse::nep245::TokenId;
use near_sdk::{AccountId, NearToken, json_types::U128};
use serde_json::json;

use crate::utils::test_log::TestLog;

pub trait DefuseMtWithdrawer {
    async fn defuse_mt_withdraw(
        &self,
        defuse_id: &AccountId,
        token_id: &AccountId,
        receiver_id: &AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<u128>,
        msg: Option<String>,
    ) -> anyhow::Result<(Vec<u128>, TestLog)>;
}

impl DefuseMtWithdrawer for near_workspaces::Account {
    async fn defuse_mt_withdraw(
        &self,
        defuse_id: &AccountId,
        token: &AccountId,
        receiver_id: &AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<u128>,
        msg: Option<String>,
    ) -> anyhow::Result<(Vec<u128>, TestLog)> {
        let (result, test_log) = self
            .call(defuse_id, "mt_withdraw")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "token": token,
                "receiver_id": receiver_id,
                "token_ids": token_ids,
                "amounts": amounts.into_iter().map(U128).collect::<Vec<_>>(),
                "msg": msg
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()
            .map(|outcome| (outcome.clone(), TestLog::from(outcome)))?;

        Ok((
            result
                .json::<Vec<U128>>()
                .map(|v| v.into_iter().map(|val| val.0).collect())?,
            test_log,
        ))
    }
}
