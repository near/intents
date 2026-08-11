#[cfg_attr(feature = "near-contract", derive(::near_sdk::FunctionError))]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("refund event log would be too long")]
pub struct ErrorLogTooLong;
