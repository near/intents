use near_account_id::AccountId;
use serde::Serialize;

use crate::AuthorizationResolution;

/// Bindings for [`AuthResolver`](crate::AuthResolver) contract interface.
#[near_kit::contract]
pub trait AuthResolverContract {
    /// See [`w_resolve_auth()`](crate::AuthResolver::w_resolve_auth) method.
    fn w_resolve_auth(&self, args: WResolveAuthArgs<'_>) -> AuthorizationResolution;
}

/// Arguments for [`w_resolve_auth()`](AuthResolverContractClient::w_resolve_auth) view-method.
#[derive(Serialize)]
pub struct WResolveAuthArgs<'a> {
    pub path: &'a [AccountId],
    pub authorization: &'a str,
}
