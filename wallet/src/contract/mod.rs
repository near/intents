mod impl_;
mod utils;

use std::collections::BTreeSet;

use near_sdk::{AccountId, FunctionError, Promise, env, near};

use crate::{
    AddExtensionOp, Error, RemoveExtensionOp, Request, RequestMessage, Result, SetSignatureModeOp,
    Wallet, WalletEvent, WalletOp, signature::SigningStandard,
};

pub use self::impl_::*;

#[near]
impl Wallet for Contract {
    #[payable]
    fn w_execute_signed(&mut self, signed: RequestMessage, proof: String) {
        self.execute_signed(signed, proof)
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
        self.public_key.to_string()
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

impl Contract {
    fn execute_signed(&mut self, signed: RequestMessage, proof: String) -> Result<()> {
        if !self.is_signature_allowed() {
            return Err(Error::SignatureDisabled);
        }

        // check chain_id
        if signed.chain_id != utils::chain_id() {
            return Err(Error::InvalidChainId {
                got: signed.chain_id,
                expected: utils::chain_id(),
            });
        }

        // check signer_id
        if signed.signer_id != env::current_account_id() {
            return Err(Error::InvalidSignerId(signed.signer_id));
        }

        // It makes sense to emit here, since checks above ensure this request
        // is intended to be relayed to this instance of wallet-contract.
        // In case following checks or request execution fail, this would
        // at least give relayers/indexers some observability on what exact
        // request hash was it.
        WalletEvent::SignedRequest {
            hash: signed.hash(),
        }
        .emit();

        // check seqno
        if signed.seqno != self.seqno {
            return Err(Error::InvalidSeqno {
                got: signed.seqno,
                expected: self.seqno,
            });
        }
        // bump seqno
        self.seqno = self
            .seqno
            .checked_add(1)
            .unwrap_or_else(|| env::panic_str("seqno overflow"));

        // check valid_until
        if signed.valid_until.has_expired() {
            return Err(Error::Expired);
        }

        // verify signature
        if !<Self as ContractImpl>::SigningStandard::verify(
            &signed.wrap_domain(),
            &self.public_key,
            &proof,
        ) {
            return Err(Error::InvalidSignature);
        }

        self.execute_request(signed.request)
    }

    fn execute_extension(&mut self, request: Request) -> Result<()> {
        if env::attached_deposit().is_zero() {
            return Err(Error::InsufficientDeposit);
        }

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
            WalletOp::SetSignatureMode(SetSignatureModeOp { enable }) => {
                self.set_signature_mode(enable)
            }
            WalletOp::AddExtension(AddExtensionOp { account_id }) => self.add_extension(account_id),
            WalletOp::RemoveExtension(RemoveExtensionOp { account_id }) => {
                self.remove_extension(account_id)
            }
            // custom ops are not supported, so we just skip them
            WalletOp::Custom(_op) => Ok(()),
        }
    }

    fn set_signature_mode(&mut self, enable: bool) -> Result<()> {
        if self.signature_enabled == enable {
            return Err(Error::ThisSignatureModeAlreadySet);
        }
        self.signature_enabled = enable;

        WalletEvent::SignatureModeSet {
            enable: self.signature_enabled,
        }
        .emit();

        self.check_lockout()
    }

    fn add_extension(&mut self, account_id: AccountId) -> Result<()> {
        if !self.extensions.insert(account_id.clone()) {
            return Err(Error::ExtensionExists(account_id));
        }

        WalletEvent::ExtensionAdded {
            account_id: account_id.into(),
        }
        .emit();

        Ok(())
    }

    fn remove_extension(&mut self, account_id: AccountId) -> Result<()> {
        if !self.extensions.remove(&account_id) {
            return Err(Error::ExtensionNotExist(account_id));
        }

        WalletEvent::ExtensionRemoved {
            account_id: account_id.into(),
        }
        .emit();

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
