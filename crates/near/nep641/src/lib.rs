//! NEP-641: Off-chain Authorization Resolution for Wallet Contracts.

use near_sdk::{AccountId, near};

#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Purpose {
    ProveOwnership,
    ApproveOffchainAction,
}

impl core::fmt::Display for Purpose {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ProveOwnership => f.write_str("PROVE_OWNERSHIP"),
            Self::ApproveOffchainAction => f.write_str("APPROVE_OFFCHAIN_ACTION"),
        }
    }
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorizationResolution {
    Resolved {
        payload: String,
    },
    Pending {
        payload: String,
        pending_authorizations: Vec<PendingAuthorization>,
    },
    Invalid {
        error_kind: ErrorKind,
        error_message: String,
    },
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct PendingAuthorization {
    pub account_id: AccountId,
    pub purpose: Purpose,
    pub authorization: String,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorKind {
    InvalidInput,
    InvalidSignature,
}
