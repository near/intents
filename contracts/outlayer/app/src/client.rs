//! Bindings for [`OutlayerApp`](crate::contract::OutlayerApp) contract.

use std::borrow::Cow;

use defuse_serde_utils::hex::AsHex;
use near_account_id::{AccountId, AccountIdRef};
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};

#[near_kit::contract]
pub trait OutlayerAppContract {
    #[call]
    fn oa_set_code(&mut self, args: OaSetCodeArgs<'_>);

    #[call]
    fn oa_transfer_admin(&mut self, args: OaTransferAdminArgs<'_>);

    fn oa_admin_id(&self) -> AccountId;
    fn oa_code_hash(&self) -> AsHex<[u8; 32]>;
    fn oa_code_url(&self) -> String;
}

/// Arguments for [`oa_set_code()`](OutlayerAppContractClient::oa_set_code) method
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct OaSetCodeArgs<'a> {
    #[serde_as(as = "Hex")]
    pub old_code_hash: [u8; 32],
    #[serde_as(as = "Hex")]
    pub new_code_hash: [u8; 32],
    pub new_code_url: Cow<'a, str>,
}

/// Arguments for [`oa_transfer_admin()`](OutlayerAppContractClient::oa_transfer_admin) method
#[derive(Serialize, Deserialize)]
pub struct OaTransferAdminArgs<'a> {
    pub new_admin_id: Cow<'a, AccountIdRef>,
}

impl From<AccountId> for OaTransferAdminArgs<'static> {
    #[inline]
    fn from(admin_id: AccountId) -> Self {
        Self {
            new_admin_id: admin_id.into(),
        }
    }
}

impl<'a> From<&'a AccountId> for OaTransferAdminArgs<'a> {
    #[inline]
    fn from(admin_id: &'a AccountId) -> Self {
        Self {
            new_admin_id: admin_id.into(),
        }
    }
}

impl<'a> From<&'a AccountIdRef> for OaTransferAdminArgs<'a> {
    #[inline]
    fn from(admin_id: &'a AccountIdRef) -> Self {
        Self {
            new_admin_id: admin_id.into(),
        }
    }
}
