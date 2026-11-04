//! Bindings for [`OutlayerApp`](crate::contract::OutlayerApp) contract.

use near_account_id::{AccountId, AccountIdRef};
use near_kit::NearToken;

pub use defuse_serde_utils::hex::AsHex;

#[near_kit::contract]
pub trait OutlayerAppContract {
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn oa_set_code(
        &mut self,
        old_code_hash: AsHex<[u8; 32]>,
        new_code_hash: AsHex<[u8; 32]>,
        new_code_url: &str,
    );

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn oa_transfer_admin(&mut self, new_admin_id: &AccountIdRef);

    fn oa_admin_id(&self) -> AccountId;
    fn oa_code_hash(&self) -> AsHex<[u8; 32]>;
    fn oa_code_url(&self) -> String;
}
