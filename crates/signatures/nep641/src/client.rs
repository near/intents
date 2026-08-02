use std::borrow::Cow;

use near_account_id::AccountIdRef;
use serde::Serialize;

use crate::AuthorizationResolution;

/// Bindings to [`OffchainAuthorizer`](crate::contract::OffchainAuthorizer)
/// contract interface.
#[near_kit::contract]
pub trait AuthResolverContract {
    fn w_resolve_auth(&self, args: WResolveAuthArgs<'_>) -> AuthorizationResolution;
}

/// Arguments for [`w_resolve_auth()`](OffchainAuthorizerContractClient::w_resolve_auth)
/// view-method.
#[derive(Serialize)]
pub struct WResolveAuthArgs<'a> {
    pub receiver_id: Cow<'a, AccountIdRef>,
    pub input: Cow<'a, str>,
}
