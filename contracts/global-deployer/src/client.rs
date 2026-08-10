//! Bindings for [`GlobalDeployer`](crate::contract::GlobalDeployer) contract.

use std::borrow::Cow;

use defuse_borsh_utils::{AsWrap, Remainder};
use defuse_serde_utils::hex::AsHex;
use near_account_id::{AccountId, AccountIdRef};
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};

#[near_kit::contract]
pub trait GlobalDeployerContract {
    #[call]
    fn gd_approve(&mut self, args: GdApproveArgs) -> bool;

    #[call]
    #[borsh]
    fn gd_deploy(&mut self, code: AsWrap<Vec<u8>, Remainder>) -> bool;

    #[call]
    fn gd_transfer_ownership(&mut self, args: GdTransferOwnershipArgs);

    fn gd_owner_id(&self) -> AccountId;
    fn gd_code_hash(&self) -> AsHex<[u8; 32]>;
    fn gd_approved_hash(&self) -> AsHex<[u8; 32]>;
}

/// Arguments for [`gd_approve()`](GlobalDeployerContract::gd_approve) method
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct GdApproveArgs {
    #[serde_as(as = "Hex")]
    pub old_hash: [u8; 32],
    #[serde_as(as = "Hex")]
    pub new_hash: [u8; 32],
}

/// Arguments for [`gd_transfer_ownership()`](GlobalDeployerContract::gd_transfer_ownership) method

#[derive(Serialize, Deserialize)]
pub struct GdTransferOwnershipArgs<'a> {
    pub receiver_id: Cow<'a, AccountIdRef>,
}

impl From<AccountId> for GdTransferOwnershipArgs<'static> {
    #[inline]
    fn from(receiver_id: AccountId) -> Self {
        Self {
            receiver_id: receiver_id.into(),
        }
    }
}

impl<'a> From<&'a AccountId> for GdTransferOwnershipArgs<'a> {
    #[inline]
    fn from(receiver_id: &'a AccountId) -> Self {
        Self {
            receiver_id: receiver_id.into(),
        }
    }
}

impl<'a> From<&'a AccountIdRef> for GdTransferOwnershipArgs<'a> {
    #[inline]
    fn from(receiver_id: &'a AccountIdRef) -> Self {
        Self {
            receiver_id: receiver_id.into(),
        }
    }
}
