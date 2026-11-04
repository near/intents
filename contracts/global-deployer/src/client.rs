//! Bindings for [`GlobalDeployer`](crate::contract::GlobalDeployer) contract.

use defuse_borsh_utils::{AsWrap, Remainder};
use defuse_serde_utils::hex::AsHex;
use near_account_id::{AccountId, AccountIdRef};
use near_kit::NearToken;

#[near_kit::contract]
pub trait GlobalDeployerContract {
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn gd_approve(&mut self, old_hash: AsHex<[u8; 32]>, new_hash: AsHex<[u8; 32]>);

    #[call]
    #[borsh]
    fn gd_deploy(&mut self, code: AsWrap<Vec<u8>, Remainder>);

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn gd_transfer_ownership(&mut self, receiver_id: &AccountIdRef);

    fn gd_owner_id(&self) -> AccountId;
    fn gd_code_hash(&self) -> AsHex<[u8; 32]>;
    fn gd_approved_hash(&self) -> AsHex<[u8; 32]>;
}
