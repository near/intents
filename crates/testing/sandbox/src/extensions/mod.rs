pub mod mt_receiver;
pub mod wnear;
// TODO: remove after near kit release
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

use crate::outcome::SuccessfulExecutionOutcome;
use anyhow::Result;
use near_kit::{AccountId, Final, FunctionCall, Gas, IntoNearToken, Near};

pub const DEFAULT_GAS: Gas = Gas::from_tgas(300);
pub const DEFAULT_DEPOSIT: Gas = Gas::from_tgas(300);

pub trait FnCallTransaction {
    async fn fn_call(
        &self,
        contract: impl Into<AccountId>,
        action: impl Into<FunctionCall>,
        deposit: impl IntoNearToken,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl FnCallTransaction for Near {
    async fn fn_call(
        &self,
        contract: impl Into<AccountId>,
        action: impl Into<FunctionCall>,
        deposit: impl IntoNearToken,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(contract.into())
            .add_action(action.into().gas(DEFAULT_GAS).deposit(deposit))
            .wait_until(Final)
            .await?
            .try_into()
    }
}
