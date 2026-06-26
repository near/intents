use std::collections::BTreeMap;

use anyhow::Result;
use defuse::core::tokens::imt::ImtTokens;
use defuse_core::amounts::Amounts;
use near_kit::{AccountId, AccountIdRef, Near, NearToken};
use serde::Serialize;

use crate::{extensions::FnCallTransaction, outcome::SuccessfulExecutionOutcome};

#[derive(Serialize)]
pub struct ImtBurnArgs<'a> {
    pub minter_id: &'a AccountIdRef,
    pub tokens: ImtTokens,
    pub memo: Option<String>,
}

#[near_kit::contract]
pub trait ImtBurnerContract {
    #[call]
    fn imt_burn(&mut self, args: ImtBurnArgs);
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

impl DefuseImtExt for Near {
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
            ImtBurnerContract::imt_burn(ImtBurnArgs {
                minter_id: minter_id.as_ref(),
                tokens,
                memo: memo.into(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }
}
