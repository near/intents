use near_sdk::Gas;

pub const MAX_TOKEN_ID_LEN: usize = 127;

pub const MT_ON_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(5);
pub const MT_ON_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(30);

#[cfg(feature = "imt")]
pub mod imt {
    use std::collections::BTreeMap;

    use defuse_token_id::{TokenId, imt::ImtTokenId};
    use near_sdk::AccountIdRef;

    use crate::{DefuseError, Result, amounts::Amounts, tokens::MAX_TOKEN_ID_LEN};

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
}
