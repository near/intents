use serde::Serialize;

use crate::{OffchainMessage, PendingAuthorization};

/// Bindings to [`OffchainAuthorizer`](crate::contract::OffchainAuthorizer)
/// contract interface.
#[near_kit::contract]
pub trait AuthResolverContract {
    fn w_resolve_auth(&self, args: WResolveAuthArgs<'_>) -> Vec<PendingAuthorization>;
}

/// Arguments for [`w_resolve_auth()`](OffchainAuthorizerContractClient::w_resolve_auth)
/// view-method.
#[derive(Serialize)]
pub struct WResolveAuthArgs<'a> {
    pub msg: &'a OffchainMessage,
    pub proof: &'a str,
}

impl<'a> From<(&'a OffchainMessage, &'a str)> for WResolveAuthArgs<'a> {
    #[inline]
    fn from((msg, proof): (&'a OffchainMessage, &'a str)) -> Self {
        Self { msg, proof }
    }
}
