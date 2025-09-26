use super::MAX_ALLOWED_TOKEN_ID_LEN;
use near_account_id::ParseAccountError;

#[derive(thiserror::Error, Debug)]
pub enum TokenIdError {
    #[error("AccountId: {0}")]
    AccountId(#[from] ParseAccountError),
    #[error(transparent)]
    ParseError(#[from] strum::ParseError),
    #[error("token_id is too long. Max length is {MAX_ALLOWED_TOKEN_ID_LEN}, got {0}")]
    TokenIdTooLarge(usize),
}
