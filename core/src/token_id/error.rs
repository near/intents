use super::MAX_ALLOWED_TOKEN_ID_LEN;
use near_account_id::ParseAccountError;

#[derive(thiserror::Error, Debug)]
pub enum TokenIdError {
    #[error("AccountId: {0}")]
    AccountId(#[from] ParseAccountError),
    #[error(transparent)]
    ParseError(#[from] strum::ParseError),
    #[error("Token id provided is too large. Given: {0}. Max: {MAX_ALLOWED_TOKEN_ID_LEN}")]
    TokenIdTooLarge(usize),
}
