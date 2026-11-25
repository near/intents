//! Bindings for [`OutlayerApp`](crate::contract::OutlayerApp) contract.

use near_account_id::{AccountId, AccountIdRef};
use near_kit_primitives::{Gas, NearToken, contract};

pub use defuse_serde_utils::hex::AsHex;

#[contract]
pub trait OutlayerAppContract {
    /// See [`oa_set_code()`](crate::contract::OutlayerApp::oa_set_code) method.
    #[call(deposit = NearToken::from_yoctonear(1), gas = Gas::from_tgas(15))]
    fn oa_set_code(
        &mut self,
        old_code_hash: AsHex<[u8; 32]>,
        new_code_hash: AsHex<[u8; 32]>,
        new_code_url: &str,
    );

    /// See [`oa_transfer_admin()`](crate::contract::OutlayerApp::oa_transfer_admin) method.
    #[call(deposit = NearToken::from_yoctonear(1), gas = Gas::from_tgas(10))]
    fn oa_transfer_admin(&mut self, new_admin_id: &AccountIdRef);

    /// See [`oa_admin_id()`](crate::contract::OutlayerApp::oa_admin_id) method.
    fn oa_admin_id(&self) -> AccountId;
    /// See [`oa_code_hash()`](crate::contract::OutlayerApp::oa_code_hash) method.
    fn oa_code_hash(&self) -> AsHex<[u8; 32]>;
    /// See [`oa_code_url()`](crate::contract::OutlayerApp::oa_code_url) method.
    fn oa_code_url(&self) -> String;
}
