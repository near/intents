#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenId(defuse_token_id::TokenId);

const MAX_ALLOWED_TOKEN_ID_LEN: usize = 127;

impl TryFrom<defuse_token_id::TokenId> for TokenId {
    type Error = TokenIdTooLarge;

    fn try_from(value: defuse_token_id::TokenId) -> Result<Self, Self::Error> {
        match &value {
            defuse_token_id::TokenId::Nep141(_) => {}
            defuse_token_id::TokenId::Nep171(token_id) => {
                if token_id.nft_token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
                    return Err(TokenIdTooLarge(token_id.nft_token_id.len()));
                }
            }
            defuse_token_id::TokenId::Nep245(token_id) => {
                if token_id.mt_token_id.len() > MAX_ALLOWED_TOKEN_ID_LEN {
                    return Err(TokenIdTooLarge(token_id.mt_token_id.len()));
                }
            }
        }
        Ok(Self(value))
    }
}

#[derive(thiserror::Error, Debug)]
#[error("token_id is too long: max length is {MAX_ALLOWED_TOKEN_ID_LEN}, got {0}")]
pub struct TokenIdTooLarge(usize);

// #[cfg(not(feature = "unbounded"))]
//     #[rstest]
//     fn token_id_length(random_bytes: Vec<u8>) {
//         let mut u = Unstructured::new(&random_bytes);
//         let contract_id = u.arbitrary_as::<_, ArbitraryAccountId>().unwrap();
//         let token_id: String = u.arbitrary().unwrap();

//         let r = Nep171TokenId::new(contract_id, token_id.clone());
//         if token_id.len() > crate::MAX_ALLOWED_TOKEN_ID_LEN {
//             assert!(matches!(r.unwrap_err(), TokenIdError::TokenIdTooLarge(_)));
//         } else {
//             r.unwrap();
//         }
//     }
