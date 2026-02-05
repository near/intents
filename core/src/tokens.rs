use near_sdk::{AccountIdRef, Gas, near};
use serde_with::{DisplayFromStr, serde_as};
use std::{borrow::Cow, collections::BTreeMap};

use crate::{amounts::Amounts, intents::tokens::Transfer};

pub const MAX_TOKEN_ID_LEN: usize = 127;

pub const MT_ON_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(5);
pub const MT_ON_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(30);

#[cfg(feature = "imt")]
pub mod imt {
    use defuse_token_id::{TokenId, imt::ImtTokenId};
    use near_sdk::{AccountIdRef, near};
    use serde_with::{DisplayFromStr, serde_as};
    use std::{borrow::Cow, collections::BTreeMap};

    use crate::{
        DefuseError, Result, amounts::Amounts, intents::tokens::imt::ImtMint,
        tokens::MAX_TOKEN_ID_LEN,
    };

    pub type ImtTokens = Amounts<BTreeMap<defuse_nep245::TokenId, u128>>;

    impl ImtTokens {
        #[inline]
        pub fn into_generic_tokens(
            self,
            minter_id: &AccountIdRef,
        ) -> Result<Amounts<BTreeMap<TokenId, u128>>> {
            let tokens = self
                .into_iter()
                .map(|(token_id, amount)| {
                    if token_id.len() > MAX_TOKEN_ID_LEN {
                        return Err(DefuseError::TokenIdTooLarge(token_id.len()));
                    }

                    let token = ImtTokenId::new(minter_id, token_id).into();

                    Ok((token, amount))
                })
                .collect::<Result<_, _>>()?;

            Ok(Amounts::new(tokens))
        }
    }

    #[near(serializers = [json])]
    #[derive(Debug, Clone)]
    pub struct ImtMintEvent<'a> {
        pub receiver_id: Cow<'a, AccountIdRef>,

        #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
        pub tokens: ImtTokens,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub memo: Cow<'a, Option<String>>,
    }

    impl<'a> From<&'a ImtMint> for ImtMintEvent<'a> {
        #[inline]
        fn from(intent: &'a ImtMint) -> Self {
            Self {
                receiver_id: Cow::Borrowed(&intent.receiver_id),
                tokens: intent.tokens.clone(),
                memo: Cow::Borrowed(&intent.memo),
            }
        }
    }
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferEvent<'a> {
    pub receiver_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    pub tokens: Amounts,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Cow<'a, Option<String>>,
}

impl<'a> From<&'a Transfer> for TransferEvent<'a> {
    #[inline]
    fn from(intent: &'a Transfer) -> Self {
        Self {
            receiver_id: Cow::Borrowed(&intent.receiver_id),
            tokens: intent.tokens.clone(),
            memo: Cow::Borrowed(&intent.memo),
        }
    }
}
