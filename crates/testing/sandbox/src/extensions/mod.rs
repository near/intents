pub mod acl;
pub mod mt_receiver;
pub mod nft;
pub mod wnear;
// TODO: remove when mt will be part of near kit extensions
pub mod mt;

#[cfg(feature = "defuse")]
pub mod defuse;
#[cfg(feature = "escrow")]
pub mod escrow;
#[cfg(feature = "deployer")]
pub mod global_deployer;
#[cfg(feature = "outlayer")]
pub mod outlayer_app;
#[cfg(feature = "poa")]
pub mod poa;
#[cfg(feature = "wallet")]
pub mod wallet;

use anyhow::Result;
use near_kit::{AccountId, SigningNear, rpc::FinalExecutionOutcome, transaction::FunctionCall};
use serde::de::DeserializeOwned;

pub trait FnCallTransaction {
    async fn fn_call(
        &self,
        contract: impl Into<AccountId>,
        action: FunctionCall,
    ) -> Result<FinalExecutionOutcome>;

    async fn fn_call_json<T: DeserializeOwned>(
        &self,
        contract: impl Into<AccountId>,
        action: FunctionCall,
    ) -> Result<(FinalExecutionOutcome, T)>;
}

impl FnCallTransaction for SigningNear {
    async fn fn_call(
        &self,
        contract: impl Into<AccountId>,
        action: FunctionCall,
    ) -> Result<FinalExecutionOutcome> {
        self.transaction(contract.into())
            .add_action(action)
            .await?
            .checked()
            .map_err(Into::into)
    }

    async fn fn_call_json<T: DeserializeOwned>(
        &self,
        contract: impl Into<AccountId>,
        action: FunctionCall,
    ) -> Result<(FinalExecutionOutcome, T)> {
        let outcome = self
            .transaction(contract.into())
            .add_action(action)
            .await?
            .checked()?;
        let value = outcome.json::<T>()?;
        Ok((outcome, value))
    }
}
