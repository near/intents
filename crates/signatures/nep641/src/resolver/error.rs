use itertools::Itertools;

use near_account_id::AccountId;

use crate::resolver::{AccessKeyError, contract::ContractError};

/// An error returned by [`crate::resolver::RpcResolver`]
#[derive(Debug, thiserror::Error)]
#[error("{}: {}", .rev_path.iter().chain([.account_id]).join(" -> "), .kind)]
#[non_exhaustive]
pub struct ResolveError {
    /// Account ID for which the resolution failed.
    pub account_id: AccountId,
    /// **REVERSED** path _starting_ from the top-level resolver.
    pub rev_path: Vec<AccountId>,
    /// The actual error occurred.
    pub kind: ResolveErrorKind,
}

/// An resolve error [kind](field@ResolveError::kind)
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ResolveErrorKind {
    #[error(transparent)]
    AccessKey(#[from] AccessKeyError),

    #[error(transparent)]
    Contract(#[from] ContractError),

    #[error("resolved payload is invalid: expected: {}, got: {}", .expected, .payload)]
    InvalidPayload { payload: String, expected: String },

    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),

    #[error("max depth exceeded, maximum is set to: {0}")]
    MaxDepthExceeded(usize),

    #[error("RPC: {0}")]
    Rpc(#[from] near_kit::RpcError),

    #[error("too many sub-authorizations, maximum is set to: {0}")]
    TooManySubAuthorizations(usize),
}

impl ResolveErrorKind {
    /// Annotate the error with path and current resolver ID
    #[must_use]
    #[inline]
    pub const fn at(self, account_id: AccountId, rev_path: Vec<AccountId>) -> ResolveError {
        ResolveError {
            account_id,
            rev_path,
            kind: self,
        }
    }
}
