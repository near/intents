use near_sdk::{AccountIdRef, NearToken, serde_json::json};

use super::ft::FtExt;
use crate::{
    SigningAccount,
    tx::{FnCallBuilder, TxResult},
};

pub trait WNearExt: FtExt {
    async fn near_deposit(&self, wnear_id: &AccountIdRef, amount: NearToken) -> TxResult<()>;
    async fn near_withdraw(&self, wnear_id: &AccountIdRef, amount: NearToken) -> TxResult<()>;
}

impl WNearExt for SigningAccount {
    async fn near_deposit(&self, wnear_id: &AccountIdRef, amount: NearToken) -> TxResult<()> {
        self.tx(wnear_id.into())
            .function_call(
                FnCallBuilder::new("near_deposit")
                    .with_deposit(NearToken::from_yoctonear(amount.as_yoctonear())),
            )
            .await?;

        Ok(())
    }

    async fn near_withdraw(&self, wnear_id: &AccountIdRef, amount: NearToken) -> TxResult<()> {
        self.tx(wnear_id.into())
            .function_call(FnCallBuilder::new("near_withdraw").json_args(&json!({
                "amount": amount,
            })))
            .await?;

        Ok(())
    }
}
