//! [`GlobalDeployer`] contract interface

use defuse_borsh_utils::{AsWrap, Remainder};
use defuse_serde_utils::hex::AsHex;
use near_account_id::AccountId;
use near_sdk::{Promise, ext_contract};

/// Minimalistic contract to manage globally deployed contract code and upgrades ownership.
#[ext_contract(ext_global_deployer)]
pub trait GlobalDeployer {
    /// Approve a future deployment of a new SHA-256 code hash. Replaces previous approval, if any.
    /// `old_hash` MUST match current [`.gd_code_hash()`](Self::gd_code_hash).
    ///
    /// Allowed only for [owner](Self::gd_owner_id). MUST attach exactly 1yN.
    fn gd_approve(&mut self, old_hash: AsHex<[u8; 32]>, new_hash: AsHex<[u8; 32]>);

    /// Deploy WASM code as a global contract by this account ID. Requires attached deposit for
    /// storage staking, refunds the rest.
    ///
    /// This method is permissionless: anyone can deploy as long as SHA-256 hash of the code
    /// matches [currently approved](Self::gd_approved_hash) one.
    ///
    /// This method accepts raw `.wasm` bytes directly — just pass the binary contents of the
    /// file as the function call input, without any borsh length prefix.
    fn gd_deploy(&mut self, #[serializer(borsh)] code: AsWrap<Vec<u8>, Remainder>) -> Promise;

    /// Transfer ownership to a new owner ID and reset [approved hash](Self::gd_approved_hash).
    ///
    /// Allowed only for [owner](Self::gd_owner_id). MUST attach exactly 1yN.
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId);

    /// Returns the current owner's account ID.
    fn gd_owner_id(&self) -> AccountId;

    /// Returns the SHA-256 hash of the currently deployed code, or `0000..000` if none.
    fn gd_code_hash(&self) -> AsHex<[u8; 32]>;

    /// Returns the next approved SHA-256 code hash, or `0000..000` if none.
    fn gd_approved_hash(&self) -> AsHex<[u8; 32]>;
}

#[cfg(feature = "_contract")]
#[cfg_attr(not(near), allow(dead_code))]
const _: () = {
    use defuse_digest::{Digest, sha2::Sha256};
    use near_account_id::AccountIdRef;
    use near_sdk::{FunctionError, Gas, NearToken, PanicOnDefault, env, near};

    use crate::{ApprovalReason, Error, GdEvent, State};

    type Result<T, E = Error> = ::core::result::Result<T, E>;

    #[near(
        contract_state(key = State::STATE_KEY),
        contract_metadata(
            standard(standard = "global-deployer", version = "1.0.0")
        )
    )]
    #[derive(PanicOnDefault)]
    #[repr(transparent)]
    pub struct Contract(State<'static>);

    #[near]
    impl Contract {
        /// Initialize a global deployer on the existing account and set admin to current account ID.
        ///
        /// It's recommended to call this method in the same receipt right after `UseGlobalContract` action.
        /// Requires exactly 1yN attached.
        #[allow(clippy::use_self)]
        #[private]
        #[init]
        pub fn gd_init() -> Self {
            if env::attached_deposit() != NearToken::from_yoctonear(1) {
                // reject FunctionCall access keys
                Error::RequireOneYocto.panic();
            }

            Self(State {
                owner_id: env::current_account_id().into(),
                code_hash: State::DEFAULT_HASH,
                approved_hash: State::DEFAULT_HASH,
            })
        }
    }

    #[near]
    impl GlobalDeployer for Contract {
        /// Approve a future deployment of a new SHA-256 code hash. Replaces previous approval, if any.
        /// `old_hash` MUST match current `.gd_code_hash()`.
        ///
        /// Allowed only for owner. MUST attach exactly 1yN.
        #[payable]
        fn gd_approve(&mut self, old_hash: AsHex<[u8; 32]>, new_hash: AsHex<[u8; 32]>) {
            self.gd_approve_internal(old_hash.into_inner(), new_hash.into_inner())
                .unwrap_or_else(|err| err.panic());
        }

        /// Deploy WASM code as a global contract by this account ID. Requires attached deposit for
        /// storage staking, refunds the rest.
        ///
        /// This method is permissionless: anyone can deploy as long as SHA-256 hash of the code
        /// matches currently approved one.
        ///
        /// This method accepts raw `.wasm` bytes directly — just pass the binary contents of the
        /// file as the function call input, without any borsh length prefix.
        #[payable]
        fn gd_deploy(&mut self, #[serializer(borsh)] code: AsWrap<Vec<u8>, Remainder>) -> Promise {
            self.deploy(code.into_inner())
                .unwrap_or_else(|err| err.panic())
        }

        /// Transfer ownership to a new owner ID and reset approved hash.
        ///
        /// Allowed only for owner. MUST attach exactly 1yN.
        #[payable]
        fn gd_transfer_ownership(&mut self, receiver_id: AccountId) {
            self.transfer_ownership(receiver_id)
                .unwrap_or_else(|err| err.panic());
        }

        /// Returns the current owner's account ID.
        fn gd_owner_id(&self) -> AccountId {
            self.0.owner_id.as_ref().to_owned()
        }

        /// Returns the SHA-256 hash of the currently deployed code, or `0000..000` if none.
        fn gd_code_hash(&self) -> AsHex<[u8; 32]> {
            self.0.code_hash.into()
        }

        /// Returns the next approved SHA-256 code hash, or `0000..000` if none.
        fn gd_approved_hash(&self) -> AsHex<[u8; 32]> {
            self.0.approved_hash.into()
        }
    }

    #[near]
    impl Contract {
        const GD_POST_DEPLOY_MIN_GAS: Gas = Gas::from_tgas(15);

        #[private]
        pub fn gd_post_deploy(
            &mut self,
            code_hash: AsHex<[u8; 32]>,
            initial_balance: NearToken,
            deploy_deposit: NearToken,
        ) {
            self.post_deploy(code_hash.into_inner(), initial_balance, deploy_deposit)
                .unwrap_or_else(|err| err.panic());
        }
    }

    impl Contract {
        fn gd_approve_internal(&mut self, old_hash: [u8; 32], new_hash: [u8; 32]) -> Result<()> {
            if env::attached_deposit() != NearToken::from_yoctonear(1) {
                return Err(Error::RequireOneYocto);
            }
            if !self.is_owner(&env::predecessor_account_id()) {
                return Err(Error::Unauthorized);
            }
            if !self.is_current_code_hash(&old_hash) {
                return Err(Error::InvalidCodeHash);
            }

            self.approve(
                new_hash,
                ApprovalReason::By(env::predecessor_account_id().into()),
            );
            Ok(())
        }

        #[inline]
        fn approve(&mut self, code_hash: [u8; 32], reason: ApprovalReason<'_>) {
            self.0.approved_hash = code_hash;
            GdEvent::Approve { code_hash, reason }.emit();
        }

        fn deploy(&self, code: Vec<u8>) -> Result<Promise> {
            let code_hash = Sha256::digest(&code).into();

            if !self.is_approved(&code_hash) {
                return Err(Error::InvalidCodeHash);
            }

            let initial_balance = env::account_balance().saturating_sub(env::attached_deposit());

            Ok(Self::ext_on(
                Promise::new(env::current_account_id())
                    // 0. In case a receipt fails, re-direct the refund to the same
                    // account which was specified as `refund_to` for current receipt.
                    .refund_to(env::refund_to_account_id())
                    // 1. Transfer attached deposit to ourselves, so that it doesn't
                    // affect our balance while in-flight. We could have attached
                    // it to `gd_post_deploy()` below, but this balance is needed
                    // for `deploy_global_contract_by_account_id` to succeed, so
                    // we add a separate transfer action before.
                    .transfer(env::attached_deposit())
                    // 2. Deploy the global contract by our account_id
                    .deploy_global_contract_by_account_id(code),
            )
            .with_static_gas(Self::GD_POST_DEPLOY_MIN_GAS)
            .with_unused_gas_weight(1)
            // 3. Call post-deploy callback **in the same receipt**
            .gd_post_deploy(code_hash.into(), initial_balance, env::attached_deposit()))
        }

        fn post_deploy(
            &mut self,
            code_hash: [u8; 32],
            initial_balance: NearToken,
            deploy_deposit: NearToken,
        ) -> Result<()> {
            // check that approved hash hasn't changed while in-flight
            if !self.is_approved(&code_hash) {
                return Err(Error::InvalidCodeHash);
            }

            self.0.code_hash = code_hash;
            GdEvent::Deploy { code_hash }.emit();

            // remove just-used approval
            self.approve(State::DEFAULT_HASH, ApprovalReason::Deploy(code_hash));

            let refund = env::account_balance()
                .saturating_sub(initial_balance)
                .min(deploy_deposit);
            if !refund.is_zero() {
                // refund the rest to `refund_to` forwarded here by `gd_deploy()`
                Promise::new(env::refund_to_account_id())
                    .transfer(refund)
                    .detach();
            }

            Ok(())
        }

        fn transfer_ownership(&mut self, new_owner_id: AccountId) -> Result<()> {
            if env::attached_deposit() != NearToken::from_yoctonear(1) {
                return Err(Error::RequireOneYocto);
            }
            if !self.is_owner(&env::predecessor_account_id()) {
                return Err(Error::Unauthorized);
            }
            if self.is_owner(&new_owner_id) {
                return Err(Error::SelfTransfer);
            }

            GdEvent::Transfer {
                old_owner_id: self.0.owner_id.as_ref().into(),
                new_owner_id: (&new_owner_id).into(),
            }
            .emit();
            self.0.owner_id = new_owner_id.clone().into();

            // remove an approval from previous owner
            self.approve(
                State::DEFAULT_HASH,
                // pretend that new owner did it by himself,
                // since he would be interested in doing it anyway
                ApprovalReason::By(new_owner_id.into()),
            );

            Ok(())
        }

        #[inline]
        fn is_approved(&self, hash: &[u8; 32]) -> bool {
            self.0.approved_hash == *hash
        }

        #[inline]
        fn is_current_code_hash(&self, hash: &[u8; 32]) -> bool {
            self.0.code_hash == *hash
        }

        #[inline]
        fn is_owner(&self, account_id: &AccountIdRef) -> bool {
            *self.0.owner_id == *account_id
        }
    }
};
