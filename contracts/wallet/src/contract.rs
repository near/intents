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
use defuse_near_promise::{NearPromise, StateInit, StateInitV1, actions::NearAction};
use defuse_time::Timestamp;
use impl_tools::autoimpl;
use near_account_id::{AccountId, AccountIdRef};
use near_sdk::{FunctionError, Promise, env, ext_contract};

pub use crate::ContractError as Error;
use crate::{
    AuthError, AuthSignerBinding, AuthorizationResolution, Request, RequestMessage,
    SignatureSchema, SignedAuthMessage, State, WalletOp,
    events::{Actor, WalletEvent},
};

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

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
    /// * `proof` is [invalid](SignatureSchema::verify) or signature is
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

    /// Resolve an off-chain authorization
    /// ([NEP-641](https://github.com/near/NEPs/blob/master/neps/nep-0641.md)).
    ///
    /// MUST be a view method: it doesn't modify contract state and is
    /// callable via `view_call` RPC without a signed transaction.
    ///
    /// The `authorization` blob is a JSON-serialized [`SignedAuthMessage`].
    ///
    /// Being a single-signer wallet, this MUST return either
    /// [`RESOLVED`](AuthorizationResolution::Resolved) or
    /// [`INVALID`](AuthorizationResolution::Invalid), never
    /// [`PENDING`](AuthorizationResolution::Pending). It MUST return
    /// `INVALID` (instead of panicking) in following cases:
    /// * `authorization` is not a valid JSON-serialized [`SignedAuthMessage`]
    /// * signature is [currently disabled](WalletOp::SetSignatureMode)
    /// * [`message.purpose`](crate::AuthMessage::purpose) or
    ///   [`message.recipient`](crate::AuthMessage::recipient) don't match the
    ///   supplied arguments
    /// * [`message.chain_id`](crate::AuthMessage::chain_id) is from another network
    /// * [`message.created_at`](crate::AuthMessage::created_at) is expired or
    ///   from the future
    /// * [`message.signer`](crate::AuthMessage::signer) binding doesn't match this
    ///   account / its current code and config
    /// * `proof` is [invalid](SignatureSchema::verify_hash)
    fn w_resolve_auth(
        &self,
        purpose: String,
        recipient: String,
        authorization: String,
    ) -> AuthorizationResolution;
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
    // TODO: simplify when https://github.com/near/borsh-rs/pull/373 is released
    #[cfg_attr(
        not(feature = "borsh-schema"),
        borsh(bound(
            serialize = "S::PublicKey: BorshSerialize",
            deserialize = "S::PublicKey: BorshDeserialize",
        ))
    )]
    #[cfg_attr(
        feature = "borsh-schema",
        borsh(
            bound(
                serialize = "S::PublicKey: BorshSerialize",
                deserialize = "S::PublicKey: BorshDeserialize",
            ),
            schema(params = "S => S::PublicKey"),
        )
    )]
    State<S::PublicKey>,
);

impl<S> Wallet for WalletImpl<S>
where
    S: SignatureSchema<PublicKey: Display + Clone + BorshSerialize>,
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

    #[inline]
    fn w_resolve_auth(
        &self,
        purpose: String,
        recipient: String,
        authorization: String,
    ) -> AuthorizationResolution {
        self.resolve_auth(&purpose, &recipient, &authorization)
            .map_or_else(Into::into, |payload| AuthorizationResolution::Resolved {
                payload,
            })
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
        if !S::verify(&self.0.public_key, &msg, proof) {
            return Err(Error::InvalidSignature);
        }

        let hash = msg.hash();
        WalletEvent::SignedRequest { hash }.emit();

        self.execute_request(msg.request, &Actor::SignedRequest(hash))
    }

    fn resolve_auth(
        &self,
        purpose: &str,
        recipient: &str,
        authorization: &str,
    ) -> Result<String, AuthError>
    where
        S::PublicKey: Clone + BorshSerialize,
    {
        let SignedAuthMessage {
            message: msg,
            proof,
        } = near_sdk::serde_json::from_str(authorization)
            .map_err(|err| AuthError::MalformedAuthorization(err.to_string()))?;

        // same policy as `execute_signed()`
        if !self.0.is_signature_allowed() {
            return Err(AuthError::SignatureDisabled);
        }

        // check purpose binding
        if msg.purpose != purpose {
            return Err(AuthError::PurposeMismatch);
        }

        // check recipient binding
        if msg.recipient != recipient {
            return Err(AuthError::RecipientMismatch);
        }

        // check chain_id
        if msg.chain_id != env::chain_id() {
            return Err(AuthError::InvalidChainId);
        }

        // check validity window: same rule as `Nonces::commit()`, sans bitmap
        let now = Timestamp::now();
        if !(now - self.0.nonces.timeout().min(msg.timeout) <= msg.created_at
            && msg.created_at <= now)
        {
            return Err(AuthError::ExpiredOrFuture);
        }

        // check signer binding
        match &msg.signer {
            AuthSignerBinding::SignerId { signer_id } => {
                if *signer_id != env::current_account_id() {
                    return Err(AuthError::SignerBindingMismatch);
                }
            }
            AuthSignerBinding::Code {
                allowed_factory_ids,
                signature_enabled,
                subwallet_id,
                timeout,
                extensions,
            } => {
                // Reconstruct the `StateInit` this account must have been
                // created with: the code identity is the code this account
                // is currently running under, the config comes from the
                // envelope, and `public_key` comes from the contract's own
                // state (and is additionally bound by the signature
                // verification below). The derived deterministic account
                // id commits to all three, so it can only match
                // `env::current_account_id()` if this envelope was
                // intended for this exact account.
                //
                // NOTE: requires near-sdk >= 5.29.0, where
                // `current_global_contract_id()` was fixed to return the
                // global contract's account id (rather than the current
                // account's own id) for GlobalByAccount deployments.
                let Some(code) = env::current_global_contract_id() else {
                    // not running under a global contract: this cannot be
                    // a deterministic wallet-contract instance
                    return Err(AuthError::SignerBindingMismatch);
                };

                // Enforce the signed canonical-factory allow-list: the code
                // this instance runs under MUST be one of the factory account
                // ids the signer committed to. This is what caps the set of
                // accounts a single signed message can authorize to one per
                // curve (see `AuthSignerBinding::Code::allowed_factory_ids`).
                // A rogue or not-yet-declared factory — anything the signer
                // did not list — is rejected here even before the derivation
                // check below.
                let near_sdk::GlobalContractId::AccountId(factory_id) = &code else {
                    // deployed by code hash, not by account id: not a
                    // canonical (by-account) factory this envelope allows
                    return Err(AuthError::SignerBindingMismatch);
                };
                if !allowed_factory_ids
                    .iter()
                    .any(|allowed| allowed.as_str() == factory_id.as_str())
                {
                    return Err(AuthError::SignerBindingMismatch);
                }

                let initial_state = State {
                    signature_enabled: *signature_enabled,
                    subwallet_id: *subwallet_id,
                    public_key: self.0.public_key.clone(),
                    nonces: crate::Nonces::new(*timeout),
                    extensions: extensions.clone(),
                };

                let state_init = StateInit::V1(StateInitV1 {
                    code,
                    data: initial_state.as_storage(),
                });

                if state_init.derive_account_id() != env::current_account_id() {
                    return Err(AuthError::SignerBindingMismatch);
                }
            }
        }

        // verify signature over the domain-separated authorization hash
        if !S::verify_hash(&self.0.public_key, &msg.hash(), &proof) {
            return Err(AuthError::InvalidSignature);
        }

        Ok(msg.payload)
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
        for op in request.internal {
            self.execute_op(op, actor.as_ref())?;
        }

        for promise in request.external {
            Self::build_promise(promise)?.detach();
        }

        Ok(())
    }

    fn execute_op(&mut self, op: WalletOp, actor: Actor<'_>) -> Result<()> {
        match op {
            WalletOp::SetSignatureMode { enable } => self.set_signature_mode(enable, actor),
            WalletOp::AddExtension { account_id } => self.add_extension(account_id, actor),
            WalletOp::RemoveExtension { account_id } => self.remove_extension(&account_id, actor),
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

    fn build_promise(p: NearPromise) -> Result<Promise> {
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

        Ok(p.build())
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
/// use defuse_wallet::{SignatureSchema, wallet};
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
///    /// Verify given proof over a 32-byte domain-separated digest in
///    /// respect to the public key and return whether verification passed.
///    ///
///    /// Used by the `w_execute_signed(msg, proof)` and
///    /// `w_resolve_auth(purpose, recipient, authorization)` contract methods.
///     fn verify_hash(public_key: &Self::PublicKey, hash: &[u8; 32], proof: &str) -> bool {
///         todo!("verify signature over `hash` in respect to the public key")
///     }
/// }
///
/// // Public key is stored in the contract's state.
/// #[near(serializers = [borsh])]
/// #[derive(Clone)]
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

            /// Resolve an off-chain authorization (NEP-641).
            ///
            /// This is a view method: it never modifies contract state and
            /// returns `INVALID` instead of panicking.
            fn w_resolve_auth(
                &self,
                purpose: ::std::string::String,
                recipient: ::std::string::String,
                authorization: ::std::string::String,
            ) -> $crate::AuthorizationResolution {
                self.0.w_resolve_auth(purpose, recipient, authorization)
            }
        }
    };
}
