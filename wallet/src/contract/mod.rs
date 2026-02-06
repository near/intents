mod utils;

use core::ops::{Deref, DerefMut};
use std::collections::BTreeSet;

use near_sdk::{AccountId, FunctionError, PanicOnDefault, Promise, borsh, env, near};

use crate::{Error, Request, Result, SignedRequest, SigningStandard, State, Wallet, WalletOp};

// TODO: features for standards
#[cfg(feature = "webauthn")]
type SS = crate::webauthn::Webauthn<crate::webauthn::EdDSA>;

#[near(contract_state(key = State::<SS>::STATE_KEY))]
#[derive(PanicOnDefault)]
pub struct Contract(State<SS>);

#[near]
impl Wallet for Contract {
    #[payable]
    fn w_execute_signed(&mut self, signed: SignedRequest) {
        self.execute_signed(signed)
            .unwrap_or_else(|err| err.panic())
    }

    #[payable]
    fn w_execute_extension(&mut self, request: Request) {
        self.execute_extension(request)
            .unwrap_or_else(|err| err.panic())
    }

    fn w_subwallet_id(&self) -> u32 {
        self.wallet_id
    }

    fn w_seqno(&self) -> u32 {
        self.seqno
    }

    fn w_is_signature_allowed(&self) -> bool {
        self.is_signature_allowed()
    }

    fn w_public_key(&self) -> String {
        // TODO
        // self.public_key
        todo!()
    }

    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool {
        self.has_extension(account_id)
    }

    fn w_extensions(&self) -> BTreeSet<AccountId> {
        self.extensions.clone()
    }

    fn w_chain_id(&self) -> String {
        // TODO: change to `env::chain_id()` when NEP-638 lands
        utils::chain_id()
    }
}

impl<S: SigningStandard> State<S> {
    fn execute_signed(&mut self, signed: SignedRequest) -> Result<()> {
        // check signer_id
        if signed.body.signer_id != env::current_account_id() {
            return Err(Error::InvalidSignerId(signed.body.signer_id));
        }

        // check chain_id
        if signed.body.chain_id != utils::chain_id() {
            return Err(Error::InvalidChainId {
                got: signed.body.chain_id,
                expected: utils::chain_id(),
            });
        }

        // check seqno
        if signed.body.seqno != self.seqno {
            return Err(Error::InvalidSeqno {
                got: signed.body.seqno,
                expected: self.seqno,
            });
        }
        // bump seqno
        // NOTE: this will panic on overflow due to `overflow-checks = true`
        self.seqno += 1;

        // check valid_until
        if signed.body.valid_until.has_expired() {
            return Err(Error::Expired);
        }

        // check signature
        {
            let msg = borsh::to_vec(&signed.body).unwrap_or_else(|_| unreachable!());

            if !S::verify(&msg, &self.public_key, &signed.proof) {
                return Err(Error::InvalidSignature);
            }
        }

        self.execute_request(signed.body.request)
    }

    fn execute_extension(&mut self, request: Request) -> Result<()> {
        self.check_extension_enabled(env::predecessor_account_id())?;

        self.execute_request(request)
    }

    fn execute_request(&mut self, request: Request) -> Result<()> {
        for op in request.ops {
            self.execute_op(op)?;
        }

        request.out.build().map(Promise::detach);

        Ok(())
    }

    fn execute_op(&mut self, op: WalletOp) -> Result<()> {
        match op {
            WalletOp::SetSignatureMode { enable } => self.set_signature_mode(enable),
            WalletOp::AddExtension { account_id } => self.add_extension(account_id),
            WalletOp::RemoveExtension { account_id } => self.remove_extension(account_id),
        }
    }

    fn set_signature_mode(&mut self, enable: bool) -> Result<()> {
        if self.signature_enabled == enable {
            return Err(Error::ThisSignatureModeAlreadySet);
        }
        self.signature_enabled = enable;
        // TODO: emit events

        self.check_lockout()
    }

    fn add_extension(&mut self, account_id: AccountId) -> Result<()> {
        if !self.extensions.insert(account_id.clone()) {
            return Err(Error::ExtensionExists(account_id));
        }
        Ok(())
    }

    fn remove_extension(&mut self, account_id: AccountId) -> Result<()> {
        if !self.extensions.remove(&account_id) {
            return Err(Error::ExtensionNotExist(account_id));
        }

        self.check_lockout()
    }

    fn check_extension_enabled(&self, account_id: AccountId) -> Result<()> {
        if !self.has_extension(&account_id) {
            return Err(Error::ExtensionNotExist(account_id));
        }
        Ok(())
    }

    fn check_lockout(&self) -> Result<()> {
        if !self.signature_enabled && self.extensions.is_empty() {
            return Err(Error::Lockout);
        }
        Ok(())
    }
}

impl Deref for Contract {
    type Target = State<SS>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Contract {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
