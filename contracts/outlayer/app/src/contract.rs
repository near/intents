//! [`OutlayerApp`] contract interface

use defuse_serde_utils::hex::AsHex;
use near_account_id::AccountId;
use near_sdk::ext_contract;

/// Outlayer application
#[ext_contract(ext_outlayer_app)]
pub trait OutlayerApp {
    /// Approve a new SHA-256 [code hash](Self::oa_code_hash) instead of the current one
    /// and set the new [URL](Self::oa_code_url) where the code binary can be fetched from.
    ///
    /// Allowed only for [admin](Self::oa_admin_id). MUST attach at least 1yN.
    /// Emits [`set_code`](crate::OaEvent::SetCode) event.
    fn oa_set_code(
        &mut self,
        old_code_hash: AsHex<[u8; 32]>,
        new_code_hash: AsHex<[u8; 32]>,
        new_code_url: String,
    );

    /// Transfer ownership to a new admin.
    ///
    /// Allowed only for [admin](Self::oa_admin_id). MUST attach exactly 1yN.
    /// Emits [`transfer_admin`](crate::OaEvent::TransferAdmin) event.
    fn oa_transfer_admin(&mut self, new_admin_id: AccountId);

    /// Returns the current admin's account ID.
    fn oa_admin_id(&self) -> AccountId;

    /// Returns the approved SHA-256 code hash.
    fn oa_code_hash(&self) -> AsHex<[u8; 32]>;

    /// Returns the URL where the code binary can be fetched from.
    fn oa_code_url(&self) -> String;
}

#[cfg(feature = "_contract")]
#[cfg_attr(not(near), allow(dead_code))]
const _: () = {
    use near_account_id::AccountIdRef;
    use near_sdk::{FunctionError, PanicOnDefault, env, near};

    use crate::{Error, OaEvent, State};

    type Result<T, E = Error> = ::core::result::Result<T, E>;

    /// Outlayer application
    #[near(
        contract_state(key = State::STATE_KEY),
        contract_metadata(
            standard(standard = "outlayer-app", version = "1.0.0")
        )
    )]
    #[derive(PanicOnDefault)]
    #[repr(transparent)]
    pub struct Contract(State<'static>);

    #[near]
    impl OutlayerApp for Contract {
        /// Approve a new SHA-256 code hash instead of the current one and set the new
        /// URL where the code binary can be fetched from.
        ///
        /// Allowed only for admin. MUST attach at least 1yN.
        #[payable]
        fn oa_set_code(
            &mut self,
            old_code_hash: AsHex<[u8; 32]>,
            new_code_hash: AsHex<[u8; 32]>,
            new_code_url: String,
        ) {
            self.set_code(
                old_code_hash.into_inner(),
                new_code_hash.into_inner(),
                new_code_url,
            )
            .unwrap_or_else(|err| err.panic());
        }

        /// Transfer ownership to a new admin.
        ///
        /// Allowed only for admin. MUST attach exactly 1yN.
        #[payable]
        fn oa_transfer_admin(&mut self, new_admin_id: AccountId) {
            self.transfer_admin(new_admin_id)
                .unwrap_or_else(|err| err.panic());
        }

        /// Returns the current admin's account ID.
        fn oa_admin_id(&self) -> AccountId {
            self.0.admin_id.as_ref().to_owned()
        }

        /// Returns the approved SHA-256 code hash.
        fn oa_code_hash(&self) -> AsHex<[u8; 32]> {
            self.0.code_hash.into()
        }

        /// Returns the URL where the code binary can be fetched from.
        fn oa_code_url(&self) -> String {
            self.0.code_url.as_ref().to_owned()
        }
    }

    impl Contract {
        fn set_code(
            &mut self,
            old_code_hash: [u8; 32],
            new_code_hash: [u8; 32],
            new_code_url: String,
        ) -> Result<()> {
            if env::attached_deposit().is_zero() {
                return Err(Error::InsufficientDeposit);
            }

            if !self.is_admin(&env::predecessor_account_id()) {
                return Err(Error::Unauthorized);
            }

            if !self.is_current_code_hash(&old_code_hash) {
                return Err(Error::WrongCodeHash);
            }

            self.0.code_hash = new_code_hash;
            self.0.code_url = new_code_url.into();
            OaEvent::SetCode {
                hash: new_code_hash,
                url: self.0.code_url.as_ref().into(),
            }
            .emit();

            Ok(())
        }

        fn transfer_admin(&mut self, new_admin_id: AccountId) -> Result<()> {
            if env::attached_deposit().is_zero() {
                return Err(Error::RequireOneYocto);
            }

            if !self.is_admin(&env::predecessor_account_id()) {
                return Err(Error::Unauthorized);
            }

            if self.is_admin(&new_admin_id) {
                return Err(Error::SelfTransfer);
            }

            OaEvent::TransferAdmin {
                old_admin_id: self.0.admin_id.as_ref().into(),
                new_admin_id: (&new_admin_id).into(),
            }
            .emit();
            self.0.admin_id = new_admin_id.into();

            Ok(())
        }

        fn is_current_code_hash(&self, hash: &[u8; 32]) -> bool {
            self.0.code_hash == *hash
        }

        fn is_admin(&self, account_id: &AccountIdRef) -> bool {
            *self.0.admin_id == *account_id
        }
    }
};
