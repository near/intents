use near_sdk::account_id::ParseAccountError;

#[derive(thiserror::Error, Debug)]
pub enum TokenIdError {
    #[error("AccountId: {0}")]
    AccountId(#[from] ParseAccountError),
    #[error(transparent)]
    ParseError(#[from] strum::ParseError),
}
