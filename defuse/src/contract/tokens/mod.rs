mod nep141;
mod nep171;
mod nep245;

use super::{Contract, intents::execute::ExecuteInspector};
use defuse_core::{Result, engine::Engine, tokens::TokenId};
use near_sdk::{AccountId, AccountIdRef, Gas};

pub const STORAGE_DEPOSIT_GAS: Gas = Gas::from_tgas(10);

impl Contract {
    pub(crate) fn deposit(
        &mut self,
        owner_id: AccountId,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
        memo: Option<&str>,
    ) -> Result<()> {
        Engine::new(self, ExecuteInspector::default()).deposit(owner_id, tokens, memo)?;

        Ok(())
    }

    pub(crate) fn withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        token_amounts: impl IntoIterator<Item = (TokenId, u128)>,
        memo: Option<&str>,
    ) -> Result<()> {
        Engine::new(self, ExecuteInspector::default()).withdraw(owner_id, token_amounts, memo)?;

        Ok(())
    }
}
