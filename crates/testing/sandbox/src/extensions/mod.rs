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
use near_kit::{AccountId, SigningNear, transaction::FunctionCall};
use serde::de::DeserializeOwned;

use crate::outcome::{CheckedOutcome, IntoChecked};

pub trait FnCallTransaction {
    async fn fn_call(
        &self,
        contract: impl Into<AccountId>,
        action: FunctionCall,
    ) -> Result<CheckedOutcome>;

    async fn fn_call_json<T: DeserializeOwned>(
        &self,
        contract: impl Into<AccountId>,
        action: FunctionCall,
    ) -> Result<(CheckedOutcome, T)>;
}

impl FnCallTransaction for SigningNear {
    async fn fn_call(
        &self,
        contract: impl Into<AccountId>,
        action: FunctionCall,
    ) -> Result<CheckedOutcome> {
        self.transaction(contract.into())
            .add_action(action)
            .await
            .into_checked()
    }

    async fn fn_call_json<T: DeserializeOwned>(
        &self,
        contract: impl Into<AccountId>,
        action: FunctionCall,
    ) -> Result<(CheckedOutcome, T)> {
        let outcome = self
            .transaction(contract.into())
            .add_action(action)
            .await
            .into_checked()?;
        let value = outcome.json::<T>()?;
        Ok((outcome, value))
    }
}
