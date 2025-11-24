use near_account_id::ParseAccountError;

#[derive(thiserror::Error, Debug)]
pub enum TokenIdError {
    #[error("AccountId: {0}")]
    AccountId(#[from] ParseAccountError),
    #[error(transparent)]
    ParseError(#[from] strum::ParseError),
    #[cfg(not(feature = "unbounded"))]
    #[error("token_id is too long. Max length is {max}, got {0}", max = super::MAX_ALLOWED_TOKEN_ID_LEN)]
    TokenIdTooLarge(usize),
}
