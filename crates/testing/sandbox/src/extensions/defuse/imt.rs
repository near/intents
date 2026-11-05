use std::collections::BTreeMap;

use anyhow::Result;
use defuse::core::tokens::imt::ImtTokens;
use defuse_core::amounts::Amounts;
use near_kit::{AccountId, AccountIdRef, Gas, NearToken, SigningNear};

use crate::{extensions::FnCallTransaction, outcome::SuccessfulExecutionOutcome};

#[near_kit::contract]
pub trait ImtBurnerContract {
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn imt_burn(&mut self, minter_id: &AccountIdRef, tokens: ImtTokens, memo: Option<String>);
}

pub trait DefuseImtExt {
    async fn defuse_imt_burn(
        &self,
        defuse: impl Into<AccountId>,
        minter_id: impl AsRef<AccountIdRef>,
        tokens: impl IntoIterator<Item = (impl Into<String>, u128)>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl DefuseImtExt for SigningNear {
    async fn defuse_imt_burn(
        &self,
        defuse: impl Into<AccountId>,
        minter_id: impl AsRef<AccountIdRef>,
        tokens: impl IntoIterator<Item = (impl Into<String>, u128)>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        let tokens: ImtTokens = Amounts::new(
            tokens
                .into_iter()
                .map(|(token, amount)| (token.into(), amount))
                .collect::<BTreeMap<String, u128>>(),
        );

        self.fn_call(
            defuse,
            ImtBurnerContract::imt_burn(minter_id.as_ref(), tokens, memo.into())
                .gas(Gas::from_tgas(30)),
        )
        .await
    }
}
