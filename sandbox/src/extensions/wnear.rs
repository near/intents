use near_sdk::{AccountIdRef, Gas, NearToken, serde_json::json};

use super::ft::FtExt;
use crate::{SigningAccount, TxResult};

pub trait WNearExt: FtExt {
    async fn near_deposit(&self, wnear_id: &AccountIdRef, amount: NearToken) -> TxResult<()>;
    async fn near_withdraw(&self, wnear_id: &AccountIdRef, amount: NearToken) -> TxResult<()>;
}

impl WNearExt for SigningAccount {
    async fn near_deposit(&self, wnear_id: &AccountIdRef, amount: NearToken) -> TxResult<()> {
        self.tx(wnear_id.into())
            .function_call_json(
                "near_deposit",
                json!({}),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(amount.as_yoctonear()),
            )
            .await
    }

    async fn near_withdraw(&self, wnear_id: &AccountIdRef, amount: NearToken) -> TxResult<()> {
        self.tx(wnear_id.into())
            .function_call_json(
                "near_withdraw",
                json!({
                                    "amount": amount,
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await
    }
}
