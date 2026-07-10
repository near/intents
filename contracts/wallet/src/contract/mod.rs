mod utils;

use std::{collections::BTreeSet, fmt::Display};

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_near_promise::{NearPromise, actions::NearAction};
use defuse_time::Timestamp;
use impl_tools::autoimpl;
use near_account_id::{AccountId, AccountIdRef};
use near_sdk::{FunctionError, Promise, env, ext_contract};

pub use crate::ContractError as Error;
use crate::{
    Request, RequestMessage, SignatureSchema, State, WalletOp,
    events::{Actor, WalletEvent},
};

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[ext_contract(ext_wallet)]
pub trait Wallet {
    /// Execute signed request message.
    ///
    /// SHOULD accept ANY attached deposit.
    ///
    /// MUST fail in case where the `msg.request` was not executed
    /// due to various reasons, including:
    ///   * `msg` data is invalid
    ///   * `proof` is invalid
    ///   * signature is disabled
    ///   * nonce is already used
    fn w_execute_signed(&mut self, msg: RequestMessage, proof: String);

    /// Execute request from an enabled extension.
    ///
    /// * SHOULD accept ANY **non-zero** attached deposit
    /// * MUST panic if zero deposit was attached
    /// * MUST panic if [`predecessor_account_id`](near_sdk::env::predecessor_account_id)
    ///   extension is not enabled
    fn w_execute_extension(&mut self, request: Request);

    /// Returns `subwallet_id`.
    fn w_subwallet_id(&self) -> u32;

    /// Returns whether authentication by signature is currently allowed.
    fn w_is_signature_allowed(&self) -> bool;

    /// Returns a string representation of the public key or authentication
    /// identity associated with this wallet's singing standard.
    fn w_public_key(&self) -> String;

    /// Returns whether extension with given `account_id` is enabled.
    /// If true, this `account_id` SHOULD be allowed to call
    /// `w_execute_extension()`.
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool;

    /// Returns a set of enabled extensions. Each returned account
    /// SHOULD be allowed to call `w_execute_extension()`.
    fn w_extensions(&self) -> BTreeSet<AccountId>;

    /// Returns a timeout, i.e. validity timespan for each nonce.
    fn w_timeout_secs(&self) -> u32;

    /// Returns a timestamp when nonces were last cleaned up.
    fn w_last_cleaned_at(&self) -> Timestamp;
}

// TODO: move to impl_.rs?
#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))]
#[autoimpl(Debug where S::PublicKey: trait)]
#[repr(transparent)]
pub struct ContractImpl<S: SignatureSchema>(
    // TODO: simplify
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

impl<S> Wallet for ContractImpl<S>
where
    S: SignatureSchema<PublicKey: Display>,
{
    fn w_execute_signed(&mut self, msg: RequestMessage, proof: String) {
        self.execute_signed(msg, &proof)
            .unwrap_or_else(|err| err.panic());
    }

    fn w_execute_extension(&mut self, request: Request) {
        self.execute_extension(request)
            .unwrap_or_else(|err| err.panic());
    }

    fn w_subwallet_id(&self) -> u32 {
        self.0.subwallet_id
    }

    fn w_is_signature_allowed(&self) -> bool {
        self.0.is_signature_allowed()
    }

    fn w_public_key(&self) -> String {
        self.0.public_key.to_string()
    }

    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool {
        self.0.has_extension(account_id)
    }

    fn w_extensions(&self) -> BTreeSet<AccountId> {
        self.0.extensions.clone()
    }

    fn w_timeout_secs(&self) -> u32 {
        self.0
            .nonces
            .timeout()
            .as_secs()
            .try_into() // it's serialized as u32 in state
            .unwrap_or_else(|_| unreachable!())
    }

    fn w_last_cleaned_at(&self) -> Timestamp {
        self.0.nonces.last_cleaned_at()
    }
}

impl<S> ContractImpl<S>
where
    S: SignatureSchema,
{
    fn execute_signed(&mut self, msg: RequestMessage, proof: &str) -> Result<()> {
        if !self.0.is_signature_allowed() {
            return Err(Error::SignatureDisabled);
        }

        // check chain_id
        if msg.chain_id != utils::chain_id() {
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

impl<S: SignatureSchema> From<State<S::PublicKey>> for ContractImpl<S> {
    #[inline]
    fn from(state: State<S::PublicKey>) -> Self {
        Self(state)
    }
}
