//! Bindings to [`AuthResolver`](crate::AuthResolver) contract.

use defuse_serde_utils::Reversed;
use near_account_id::AccountId;
use serde::Serialize;
use serde_with::serde_as;

use crate::AuthorizationResolution;

/// Bindings for [`AuthResolver`](crate::AuthResolver) contract interface.
#[near_kit::contract]
pub trait AuthResolverContract {
    /// See [`w_resolve_auth()`](crate::AuthResolver::w_resolve_auth) method.
    fn w_resolve_auth(&self, args: WResolveAuthArgs<'_>) -> AuthorizationResolution;
}

/// Arguments for [`w_resolve_auth()`](AuthResolverContractClient::w_resolve_auth) view-method.
#[serde_as]
#[derive(Serialize)]
pub struct WResolveAuthArgs<'a> {
    /// **REVERSED** path (will be serialized backwards)
    #[serde_as(as = "Reversed")]
    #[serde(rename = "path")]
    pub rev_path: &'a [AccountId],

    pub authorization: &'a str,
}
