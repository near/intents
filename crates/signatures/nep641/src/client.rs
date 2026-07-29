use serde::Serialize;

use crate::OffchainAuthorization;

/// Bindings to [`OffchainAuthorizer`](crate::contract::OffchainAuthorizer)
/// contract interface.
#[near_kit::contract]
pub trait OffchainAuthorizerContract {
    fn w_resolve_auth(&self, args: WResolveAuthArgs<'_>) -> Vec<OffchainAuthorization>;
}

/// Arguments for [`w_resolve_auth()`](OffchainAuthorizerContractClient::w_resolve_auth)
/// view-method.
#[derive(Serialize)]
pub struct WResolveAuthArgs<'a> {
    pub auth: &'a OffchainAuthorization,
}

impl<'a> From<&'a OffchainAuthorization> for WResolveAuthArgs<'a> {
    #[inline]
    fn from(auth: &'a OffchainAuthorization) -> Self {
        Self { auth }
    }
}
