//! This module contains [`Wallet`] contract interface definition and its
//! [reference implementation](WalletImpl).
//!
//! [`ext_wallet`] module provies a typed API for third-party contracts
//! (e.g. extensions) to construct cross-contract calls (i.e. promises)
//! to wallet contracts.
//!
//! See [`wallet!`](macro@crate::wallet) macro to define and implement wallet
//! contract variants.

use std::{collections::BTreeSet, fmt::Display};

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_near_promise::{NearPromise, actions::NearAction};
use defuse_nep641::{AuthResolver, AuthorizationResolution};
use defuse_time::Timestamp;
use impl_tools::autoimpl;
use near_account_id::{AccountId, AccountIdRef};
use near_sdk::{FunctionError, GasWeight, NearToken, Promise, env, ext_contract};

pub use crate::ContractError as Error;
use crate::{
    FT_METADATA_CALLBACK_GAS, FT_METADATA_GAS, Request, RequestMessage, SignatureSchema, State,
    WalletAuthorization, WalletOp,
    events::{Actor, WalletEvent},
};

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

/// Longest `ft_metadata()` result read by the wallet; longer results fail
/// the check.
pub const MAX_FT_METADATA_LEN: usize = 32 * 1024;

/// Token, decimals and symbol of each [`WalletOp::RequireFtMetadata`].
type FtMetadataRequirements = Vec<(AccountId, u8, String)>;

/// Wallet contract interface.
///
/// See:
/// * [`wallet!`](macro@crate::wallet) macro to define and implement wallet
///   contract variants
/// * [`ext_wallet`] to construct typed cross-contract calls (i.e. promises)
///   to wallet contracts from third-party contracts (e.g. extensions)
/// * [crate documentation](crate) for an overview of Wallet Contracts
#[ext_contract(ext_wallet)]
pub trait Wallet {
    /// Execute a signed request message.
    ///
    /// SHOULD be `#[payable]` and accept ANY attached deposit.
    ///
    /// MUST panic in following cases:
    /// * [`msg.chain_id`](RequestMessage::chain_id) is from another network
    /// * [`msg.signer_id`](RequestMessage::signer_id) doesn't match
    ///   [`env::current_account_id()`](near_sdk::env::current_account_id)
    /// * [`msg.nonce`](RequestMessage::nonce) is already used, expired or
    ///   from the future
    /// * `proof` is [invalid](SignatureSchema::verify_request_msg) or signature is
    ///   [currently disabled](WalletOp::SetSignatureMode)
    fn w_execute_signed(&mut self, msg: RequestMessage, proof: String);

    /// Execute a request from an [enabled extension](WalletOp::AddExtension).
    ///
    /// SHOULD be `#[payable]` and accept ANY **non-zero** attached deposit.
    ///
    /// MUST panic in following cases:
    /// * zero deposit was attached
    /// * [`env::predecessor_account_id()`](near_sdk::env::predecessor_account_id)
    ///   extension is not enabled
    fn w_execute_extension(&mut self, request: Request);

    /// Returns [`subwallet_id`](field@State::subwallet_id).
    fn w_subwallet_id(&self) -> u32;

    /// Returns whether authentication by signature is currently allowed.
    fn w_is_signature_allowed(&self) -> bool;

    /// Returns a string representation of the wallet's public key
    /// (or other authentication identity).
    fn w_public_key(&self) -> String;

    /// Returns whether an extension with given `account_id` is currently
    /// enabled. If true, this `account_id` SHOULD be allowed to call
    /// [`w_execute_extension()`](Self::w_execute_extension).
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool;

    /// Returns a set of currently enabled extensions. Each returned account
    /// id SHOULD be allowed to call [`w_execute_extension()`](Self::w_execute_extension).
    fn w_extensions(&self) -> BTreeSet<AccountId>;

    /// Returns a timeout (in seconds), i.e. maximum validity timespan for
    /// each nonce.
    fn w_timeout_secs(&self) -> u32;

    /// Returns a timestamp when nonces were last cleaned up.
    fn w_last_cleaned_at(&self) -> Timestamp;
}

/// Reference implementation of [`Wallet`] standard, generic over the underlying
/// [signature schema](SignatureSchema) being used.
///
/// See [`wallet!`](macro@crate::wallet) macro to define and implement your
/// own wallet contract variant.
#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))]
#[autoimpl(Debug where S::PublicKey: trait)]
#[repr(transparent)]
pub struct WalletImpl<S: SignatureSchema>(
    #[borsh(bound(
        serialize = "S::PublicKey: BorshSerialize",
        deserialize = "S::PublicKey: BorshDeserialize",
    ))]
    #[cfg_attr(feature = "borsh-schema", borsh(schema(params = "S => S::PublicKey"),))]
    State<S::PublicKey>,
);

impl<S> Wallet for WalletImpl<S>
where
    S: SignatureSchema<PublicKey: Display>,
{
    #[inline]
    fn w_execute_signed(&mut self, msg: RequestMessage, proof: String) {
        self.execute_signed(msg, &proof)
            .unwrap_or_else(|err| err.panic());
    }

    #[inline]
    fn w_execute_extension(&mut self, request: Request) {
        self.execute_extension(request)
            .unwrap_or_else(|err| err.panic());
    }

    #[inline]
    fn w_subwallet_id(&self) -> u32 {
        self.0.subwallet_id
    }

    #[inline]
    fn w_is_signature_allowed(&self) -> bool {
        self.0.is_signature_allowed()
    }

    #[inline]
    fn w_public_key(&self) -> String {
        self.0.public_key.to_string()
    }

    #[inline]
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool {
        self.0.has_extension(account_id)
    }

    #[inline]
    fn w_extensions(&self) -> BTreeSet<AccountId> {
        self.0.extensions.clone()
    }

    #[inline]
    fn w_timeout_secs(&self) -> u32 {
        self.0
            .nonces
            .timeout()
            .as_secs()
            .try_into() // it's serialized as u32 in state
            .unwrap_or_else(|_| unreachable!())
    }

    #[inline]
    fn w_last_cleaned_at(&self) -> Timestamp {
        self.0.nonces.last_cleaned_at()
    }
}

impl<S> WalletImpl<S>
where
    S: SignatureSchema,
{
    fn execute_signed(&mut self, msg: RequestMessage, proof: &str) -> Result<()> {
        if !self.0.is_signature_allowed() {
            return Err(Error::SignatureDisabled);
        }

        // TODO: change to the following when External Contract Calls land:
        // if !msg.pay_for_gas && env::is_external() {
        //     return Err(Error::UnauthorizedGasPayment);
        // }
        if msg.pay_for_gas {
            env::panic_str("`pay_for_gas` is not currently supported");
        }

        // check chain_id
        if msg.chain_id != env::chain_id() {
            return Err(Error::InvalidChainId);
        }

        // check signer_id
        if msg.signer_id != env::current_account_id() {
            return Err(Error::InvalidSignerId(msg.signer_id));
        }

        // commit the nonce
        self.0
            .nonces
            .commit(msg.nonce, msg.created_at, msg.timeout)?;

        // verify signature
        if !S::verify_request_msg(&self.0.public_key, &msg, proof) {
            return Err(Error::InvalidSignature);
        }

        let hash = msg.hash();
        WalletEvent::SignedRequest { hash }.emit();

        self.execute_request(msg.request, &Actor::SignedRequest(hash))
    }

    fn execute_extension(&mut self, request: Request) -> Result<()> {
        if env::attached_deposit().is_zero() {
            return Err(Error::InsufficientDeposit);
        }

        // check whether extension is enabled
        let extension_id = env::predecessor_account_id();
        self.check_extension_enabled(&extension_id)?;

        // maybe cleanup nonces from the storage as best-effort to make it
        // available for further applying wallet-ops below
        self.0.nonces.check_cleanup();

        self.execute_request(request, &Actor::Extension(extension_id.into()))
    }

    fn execute_request(&mut self, request: Request, actor: &Actor<'_>) -> Result<()> {
        let mut ft_metadata = FtMetadataRequirements::new();
        for op in request.internal {
            self.execute_op(op, actor.as_ref(), &mut ft_metadata)?;
        }

        for promise in request.external {
            Self::check_promise(&promise)?;
            match ft_metadata
                .iter()
                .find(|(token_id, ..)| *token_id == promise.receiver_id)
            {
                Some((_, decimals, symbol)) => {
                    promise_after_ft_metadata(&promise, *decimals, symbol)?
                }
                None => promise.build(),
            }
            .detach();
        }

        Ok(())
    }

    fn execute_op(
        &mut self,
        op: WalletOp,
        actor: Actor<'_>,
        ft_metadata: &mut FtMetadataRequirements,
    ) -> Result<()> {
        match op {
            WalletOp::SetSignatureMode { enable } => self.set_signature_mode(enable, actor),
            WalletOp::AddExtension { account_id } => self.add_extension(account_id, actor),
            WalletOp::RemoveExtension { account_id } => self.remove_extension(&account_id, actor),
            WalletOp::RequireFtMetadata {
                token_id,
                decimals,
                symbol,
            } => {
                if ft_metadata.iter().any(|(other, ..)| *other == token_id) {
                    return Err(Error::DuplicateFtMetadata(token_id));
                }
                ft_metadata.push((token_id, decimals, symbol));
                Ok(())
            }
        }
    }

    fn set_signature_mode(&mut self, enable: bool, actor: Actor<'_>) -> Result<()> {
        if self.0.signature_enabled == enable {
            return Err(Error::ThisSignatureModeAlreadySet);
        }
        self.0.signature_enabled = enable;
        self.check_lockout()?;

        WalletEvent::SignatureModeSet {
            enabled: enable,
            by: actor,
        }
        .emit();

        Ok(())
    }

    fn add_extension(&mut self, account_id: AccountId, actor: Actor<'_>) -> Result<()> {
        if !self.0.extensions.insert(account_id.clone()) {
            return Err(Error::ExtensionEnabled(account_id));
        }

        WalletEvent::ExtensionAdded {
            account_id: account_id.into(),
            by: actor,
        }
        .emit();

        Ok(())
    }

    fn remove_extension(&mut self, account_id: &AccountIdRef, actor: Actor<'_>) -> Result<()> {
        if !self.0.extensions.remove(account_id) {
            return Err(Error::ExtensionNotEnabled(account_id.to_owned()));
        }
        self.check_lockout()?;

        WalletEvent::ExtensionRemoved {
            account_id: account_id.into(),
            by: actor,
        }
        .emit();

        Ok(())
    }

    #[inline]
    fn check_extension_enabled(&self, account_id: &AccountIdRef) -> Result<()> {
        if !self.0.has_extension(account_id) {
            return Err(Error::ExtensionNotEnabled(account_id.to_owned()));
        }
        Ok(())
    }

    #[inline]
    fn check_lockout(&self) -> Result<()> {
        if !self.0.signature_enabled && self.0.extensions.is_empty() {
            return Err(Error::Lockout);
        }
        Ok(())
    }

    fn check_promise(p: &NearPromise) -> Result<()> {
        // check for no self-calls
        if p.receiver_id == env::current_account_id() {
            return Err(Error::SelfCallsNotAllowed);
        }

        // check for no unsupported actions
        if !p.actions.iter().all(|a| {
            matches!(
                a,
                NearAction::FunctionCall(_)
                    | NearAction::Transfer(_)
                    | NearAction::DeterministicStateInit(_)
            )
        }) {
            // There is no support for other actions, since they operate on
            // the account itself (e.g. `DeployContract`, `AddKey` and
            // etc...) or on its subaccounts (e.g. `CreateAccount`).
            // Wallet-contracts are not self-upgradable and do not allow
            // creating subaccounts.
            return Err(Error::UnsupportedPromiseAction);
        }

        Ok(())
    }

    /// Callback of [`WalletOp::RequireFtMetadata`]: create `promise` only if
    /// the `ft_metadata()` result has exactly these `decimals` and `symbol`.
    #[doc(hidden)]
    pub fn w_on_ft_metadata(decimals: u8, symbol: &str, promise: NearPromise) {
        Self::on_ft_metadata(decimals, symbol, promise).unwrap_or_else(|err| err.panic());
    }

    fn on_ft_metadata(decimals: u8, symbol: &str, promise: NearPromise) -> Result<()> {
        #[derive(::serde::Deserialize)]
        struct FtMetadata {
            decimals: u8,
            symbol: String,
        }

        let metadata = env::promise_result_checked(0, MAX_FT_METADATA_LEN)
            .ok()
            .and_then(|data| serde_json::from_slice::<FtMetadata>(&data).ok())
            .ok_or(Error::FtMetadataMismatch)?;
        if metadata.decimals != decimals || metadata.symbol != symbol {
            return Err(Error::FtMetadataMismatch);
        }
        Self::check_promise(&promise)?;
        promise.build().detach();
        Ok(())
    }
}

/// Call the token's `ft_metadata()`, then create `p` in
/// `w_on_ft_metadata()` if it has these `decimals` and `symbol`.
///
/// Unused gas is shared as without the check: `ft_metadata()` gets only
/// its static gas, and the callback gets the total weight of `p`'s calls.
fn promise_after_ft_metadata(p: &NearPromise, decimals: u8, symbol: &str) -> Result<Promise> {
    let weight = p
        .actions
        .iter()
        .filter_map(|action| match action {
            NearAction::FunctionCall(call) => Some(call.gas_weight),
            _ => None,
        })
        .try_fold(0, u64::checked_add)
        .ok_or(Error::GasWeightOverflow)?;
    let gas = p.estimate_gas().saturating_add(FT_METADATA_CALLBACK_GAS);
    let check = Promise::new(p.receiver_id.clone()).function_call(
        "ft_metadata",
        Vec::new(),
        NearToken::ZERO,
        FT_METADATA_GAS,
    );
    let args = borsh::to_vec(&(decimals, symbol, p))
        .unwrap_or_else(|_| unreachable!("borsh: failed to serialize"));
    Ok(check.then(
        Promise::new(env::current_account_id()).function_call_weight(
            "w_on_ft_metadata",
            args,
            NearToken::ZERO,
            gas,
            GasWeight(weight),
        ),
    ))
}

impl<S> AuthResolver for WalletImpl<S>
where
    S: SignatureSchema,
{
    #[inline]
    fn w_resolve_auth(
        &self,
        path: Vec<AccountId>,
        authorization: String,
    ) -> AuthorizationResolution {
        self.resolve_auth(&path, &authorization)
            .unwrap_or_else(|err| err.panic())
    }
}

impl<S> WalletImpl<S>
where
    S: SignatureSchema,
{
    fn resolve_auth(
        &self,
        path: &[AccountId],
        authorization: &str,
    ) -> Result<AuthorizationResolution> {
        let input: WalletAuthorization = serde_json::from_str(authorization)?;

        Ok(match input {
            WalletAuthorization::Signature { msg, proof } => {
                if !self.0.is_signature_allowed() {
                    return Err(Error::SignatureDisabled);
                }

                // check chain_id
                if msg.chain_id != env::chain_id() {
                    return Err(Error::InvalidChainId);
                }

                // check signer_id
                if msg.signer_id != env::current_account_id() {
                    return Err(Error::InvalidSignerId(msg.signer_id));
                }

                // check path
                if msg.path != path {
                    return Err(Error::InvalidPath);
                }

                // check timestamp
                if Timestamp::now() < msg.timestamp {
                    return Err(Error::FromTheFuture);
                }

                // verify signature
                if !S::verify_offchain_msg(&self.0.public_key, &msg, &proof) {
                    return Err(Error::InvalidSignature);
                }

                // authorize the payload
                AuthorizationResolution::new(msg.payload)
            }
            WalletAuthorization::Extension {
                account_id,
                authorization,
                payload,
            } => {
                // check whether extension is enabled
                self.check_extension_enabled(&account_id)?;

                // authorize the payload if and only if the extension authorizes the same one
                AuthorizationResolution::new(payload.clone()).add_pending(
                    account_id,
                    authorization,
                    payload,
                )
            }
        })
    }
}

impl<S: SignatureSchema> From<State<S::PublicKey>> for WalletImpl<S> {
    #[inline]
    fn from(state: State<S::PublicKey>) -> Self {
        Self(state)
    }
}

/// Define a contract variant and implement [`Wallet`] for it by delegating to
/// [reference implementation](WalletImpl).
///
/// # Example
///
/// ```rust
/// # use core::fmt::{self, Display};
/// use defuse_wallet::{RequestMessage, SignatureSchema, wallet, offchain::OffchainMessage};
/// use near_sdk::near;
///
/// // Define the contract struct and impl
/// wallet! {
///     #[wallet(
///         // will be used to verify the signature
///         schema = MySchema,
///         // will be propagated to `#[near(contract_metadata(...))]`
///         metadata(
///             standard(standard = "wallet-<SCHEMA>", version = "0.1.0"),
///         ),
///     )]
///     // `_` will be replaced by `WalletImpl<MySchema>`
///     struct MyContract(_);
/// }
///
/// /// Signature schema used by the wallet contract variant.
/// pub struct MySchema;
/// impl SignatureSchema for MySchema {
///     /// Public key stored in the contract's state.
///     type PublicKey = MyPublicKey;
///
///    /// Verify given proof over the request message in respect to the public
///    /// key and return whether verification passed.
///    ///
///    /// Used by the `w_execute_signed(msg, proof)` contract method.
///     fn verify_request_msg(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
///         todo!("verify signature over `msg` in respect to the public key")
///     }
///
///    /// Verify given proof over the NEP-641 offchain message in respect to the public key
///    /// and return whether verification passed.
///    ///
///    /// Used by the `w_resolve_auth(path, authorization)` contract method.
///     fn verify_offchain_msg(public_key: &Self::PublicKey, msg: &OffchainMessage, proof: &str) -> bool {
///         todo!("verify signature over `msg` in respect to the public key")
///     }
/// }
///
/// // Public key is stored in the contract's state.
/// #[near(serializers = [borsh])]
/// pub struct MyPublicKey([u8; 64]);
///
/// // `Display` is needed for `w_public_key()` contract method.
/// impl Display for MyPublicKey {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "<CURVE>:{}", bs58::encode(&self.0).into_string())
///     }
/// }
/// ```
#[macro_export]
macro_rules! wallet {
    (
        #[wallet(
            schema = $schema:ty,
            metadata($($metadata:meta),+ $(,)?) $(,)?
        )]
        $(#[$attrs:meta])*
        $vis:vis struct $contract:ident(_);
    ) => {
        #[$crate::near_sdk::near(
            contract_state(key = $crate::STATE_KEY),
            contract_metadata(
                standard(standard = "wallet", version = "1.0.0"),
                standard(standard = "nep641", version = "1.0.0"),
                $($metadata),+
            )
        )]
        $(#[$attrs])*
        #[derive($crate::near_sdk::PanicOnDefault)]
        #[repr(transparent)]
        $vis struct $contract($crate::contract::WalletImpl<$schema>);

        #[$crate::near_sdk::near]
        impl $crate::contract::Wallet for $contract {
            /// Execute a signed request message.
            ///
            /// SHOULD be `#[payable]` and accept ANY attached deposit.
            #[payable]
            #[deny_unknown_arguments]
            fn w_execute_signed(
                &mut self,
                msg: $crate::RequestMessage,
                proof: ::std::string::String,
            ) {
                self.0.w_execute_signed(msg, proof);
            }

            /// Execute a request from an enabled extension.
            ///
            /// Requires at least 1yN attached.
            #[payable]
            #[deny_unknown_arguments]
            fn w_execute_extension(&mut self, request: $crate::Request) {
                self.0.w_execute_extension(request);
            }

            /// Returns `subwallet_id.
            fn w_subwallet_id(&self) -> u32 {
                self.0.w_subwallet_id()
            }

            /// Returns whether authentication by signature is currently allowed.
            fn w_is_signature_allowed(&self) -> bool {
                self.0.w_is_signature_allowed()
            }

            /// Returns a string representation of the wallet's public key
            /// (or other authentication identity).
            fn w_public_key(&self) -> ::std::string::String {
                self.0.w_public_key()
            }

            /// Returns whether an extension with given `account_id` is
            /// currently enabled. If true, this `account_id` SHOULD be
            /// allowed to call `w_execute_extension()`.
            fn w_is_extension_enabled(&self, account_id: $crate::AccountId) -> bool {
                self.0.w_is_extension_enabled(account_id)
            }

            /// Returns a set of currently enabled extensions. Each returned
            /// account id SHOULD be allowed to call `w_execute_extension()`.
            fn w_extensions(&self) -> ::std::collections::BTreeSet<$crate::AccountId> {
                self.0.w_extensions()
            }

            /// Returns a timeout (in seconds), i.e. maximum validity
            /// timespan for each nonce.
            fn w_timeout_secs(&self) -> u32 {
                self.0.w_timeout_secs()
            }

            /// Returns a timestamp when nonces were last cleaned up.
            fn w_last_cleaned_at(&self) -> $crate::Timestamp {
                self.0.w_last_cleaned_at()
            }
        }

        #[$crate::near_sdk::near]
        impl $crate::offchain::AuthResolver for $contract {
            /// Resolve offchain authorization.
            #[deny_unknown_arguments]
            fn w_resolve_auth(
                &self,
                path: ::std::vec::Vec<$crate::AccountId>,
                authorization: ::std::string::String,
            ) -> $crate::offchain::AuthorizationResolution {
                self.0.w_resolve_auth(path, authorization)
            }
        }

        #[$crate::near_sdk::near]
        impl $contract {
            /// Callback of `WalletOp::RequireFtMetadata`: create `promise`
            /// only if the token's `ft_metadata()` has exactly these
            /// `decimals` and `symbol`.
            #[private]
            pub fn w_on_ft_metadata(
                &self,
                #[serializer(borsh)] decimals: u8,
                #[serializer(borsh)] symbol: ::std::string::String,
                #[serializer(borsh)] promise: $crate::NearPromise,
            ) {
                $crate::contract::WalletImpl::<$schema>::w_on_ft_metadata(decimals, &symbol, promise);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use defuse_near_promise::actions::FunctionCall;
    use near_sdk::{
        mock::MockAction,
        test_utils::{VMContextBuilder, get_created_receipts},
        testing_env,
    };

    use super::*;

    #[test]
    fn ft_metadata_check_keeps_gas_weights() {
        testing_env!(
            VMContextBuilder::new()
                .current_account_id("wallet.near".parse().unwrap())
                .build()
        );
        let p = NearPromise::new("token.near".parse::<AccountId>().unwrap())
            .function_call(FunctionCall::name("ft_transfer").unused_gas_weight(2))
            .transfer(NearToken::from_yoctonear(1))
            .function_call(FunctionCall::name("other").unused_gas_weight(997));
        promise_after_ft_metadata(&p, 6, "USDC").unwrap().detach();

        let calls: Vec<_> = get_created_receipts()
            .into_iter()
            .flat_map(|receipt| receipt.actions)
            .filter_map(|action| match action {
                MockAction::FunctionCallWeight {
                    method_name,
                    prepaid_gas,
                    gas_weight,
                    ..
                } => Some((
                    String::from_utf8(method_name).unwrap(),
                    prepaid_gas,
                    gas_weight.0,
                )),
                _ => None,
            })
            .collect();
        assert_eq!(
            calls,
            [
                ("ft_metadata".to_owned(), FT_METADATA_GAS, 0),
                (
                    "w_on_ft_metadata".to_owned(),
                    p.estimate_gas().saturating_add(FT_METADATA_CALLBACK_GAS),
                    999,
                ),
            ]
        );
    }

    #[test]
    fn ft_metadata_check_rejects_weight_overflow_before_creating_receipts() {
        testing_env!(VMContextBuilder::new().build());
        let p = NearPromise::new("token.near".parse::<AccountId>().unwrap())
            .function_call(FunctionCall::name("first").unused_gas_weight(u64::MAX))
            .function_call(FunctionCall::name("second").unused_gas_weight(1));
        assert!(matches!(
            promise_after_ft_metadata(&p, 6, "USDC"),
            Err(Error::GasWeightOverflow)
        ));
        assert!(get_created_receipts().is_empty());
    }
    struct RejectSignatures;
    impl SignatureSchema for RejectSignatures {
        type PublicKey = ();
        fn verify_request_msg((): &(), _: &RequestMessage, _: &str) -> bool {
            false
        }
        fn verify_offchain_msg((): &(), _: &defuse_nep641::OffchainMessage, _: &str) -> bool {
            false
        }
    }

    #[test]
    fn metadata_result_failures_never_create_the_transfer() {
        use near_sdk::{PromiseResult, RuntimeFeesConfig, test_vm_config};
        let promise = NearPromise::new("recipient.near".parse::<AccountId>().unwrap())
            .transfer(NearToken::from_yoctonear(1));
        for data in [
            b"not JSON".to_vec(),
            br#"{"decimals":6,"symbol":"USDC","symbol":"FAKE"}"#.to_vec(),
            br#"{"decimals":6,"decimals":24,"symbol":"USDC"}"#.to_vec(),
            br#"{"decimals":24,"symbol":"USDC"}"#.to_vec(),
            br#"{"decimals":6,"symbol":"FAKE"}"#.to_vec(),
            vec![b' '; MAX_FT_METADATA_LEN + 1],
        ]
        .into_iter()
        .map(PromiseResult::Successful)
        .chain([PromiseResult::Failed])
        {
            testing_env!(
                VMContextBuilder::new().build(),
                test_vm_config(),
                RuntimeFeesConfig::test(),
                std::collections::HashMap::default(),
                vec![data]
            );
            assert!(matches!(
                WalletImpl::<RejectSignatures>::on_ft_metadata(6, "USDC", promise.clone()),
                Err(Error::FtMetadataMismatch)
            ));
            assert!(get_created_receipts().is_empty());
        }
        testing_env!(
            VMContextBuilder::new().build(),
            test_vm_config(),
            RuntimeFeesConfig::test(),
            std::collections::HashMap::default(),
            vec![PromiseResult::Successful(
                br#"{"decimals":6,"symbol":"USDC"}"#.to_vec()
            )]
        );
        WalletImpl::<RejectSignatures>::on_ft_metadata(6, "USDC", promise).unwrap();
        assert_eq!(get_created_receipts().len(), 1);
    }
}
