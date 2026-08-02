use near_account_id::AccountId;
use serde::Serialize;

use crate::AuthorizationResolution;

/// Bindings to [`AuthResolver`](crate::AuthResolver) contract interface.
#[near_kit::contract]
pub trait AuthResolverContract {
    fn w_resolve_auth(&self, args: WResolveAuthArgs<'_>) -> AuthorizationResolution;
}

/// Arguments for [`w_resolve_auth()`](AuthResolverContractClient::w_resolve_auth) view-method.
#[derive(Serialize)]
pub struct WResolveAuthArgs<'a> {
    pub path: &'a [AccountId],
    pub input: &'a str,
}
