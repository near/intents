//! Bindings for [`GlobalDeployer`](crate::contract::GlobalDeployer) contract.

use defuse_borsh_utils::{AsWrap, Remainder};
use defuse_serde_utils::hex::AsHex;
use near_account_id::{AccountId, AccountIdRef};
use near_kit::NearToken;

#[near_kit::contract]
pub trait GlobalDeployerContract {
    /// See [`gd_approve()`](crate::contract::GlobalDeployer::gd_approve) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn gd_approve(&mut self, old_hash: AsHex<[u8; 32]>, new_hash: AsHex<[u8; 32]>);

    /// See [`gd_deploy()`](crate::contract::GlobalDeployer::gd_deploy) method.
    #[call]
    #[borsh]
    fn gd_deploy(&mut self, code: AsWrap<Vec<u8>, Remainder>);

    /// See [`gd_transfer_ownership()`](crate::contract::GlobalDeployer::gd_transfer_ownership) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn gd_transfer_ownership(&mut self, receiver_id: &AccountIdRef);

    /// See [`gd_owner_id()`](crate::contract::GlobalDeployer::gd_owner_id) method.
    fn gd_owner_id(&self) -> AccountId;
    /// See [`gd_code_hash()`](crate::contract::GlobalDeployer::gd_code_hash) method.
    fn gd_code_hash(&self) -> AsHex<[u8; 32]>;
    /// See [`gd_approved_hash()`](crate::contract::GlobalDeployer::gd_approved_hash) method.
    fn gd_approved_hash(&self) -> AsHex<[u8; 32]>;
}
