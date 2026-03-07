mod impl_;
mod utils;

pub use self::impl_::*;

use std::collections::BTreeSet;

use near_sdk::{AccountId, AccountIdRef, env, near};

use crate::{
    Actor, Error, Nonces, Request, RequestMessage, Wallet, WalletEvent, WalletOp,
    signature::SigningStandard,
};

#[near]
impl Wallet for Contract {
    #[payable]
    fn w_execute_extension(&mut self, request: Request) {
        self.execute_extension(request)
            .unwrap_or_else(|err| utils::panic_msg(err));
    }

    fn w_subwallet_id(&self) -> u32 {
        self.wallet_id
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
        utils::chain_id()
    }
}

#[cfg(not(feature = "highload"))]
const _: () = {
    use crate::{SeqnoNonce, WalletSeqno};

    #[near]
    impl WalletSeqno for Contract {
        #[payable]
        fn w_execute_signed(&mut self, msg: RequestMessage<SeqnoNonce>, proof: String) {
            self.execute_signed(msg, &proof)
                .unwrap_or_else(|err| utils::panic_msg(err));
        }

        fn w_seqno(&self) -> u32 {
            self.nonces.seqno
        }
    }
};

#[cfg(feature = "highload")]
const _: () = {
    use crate::{TimeoutNonce, WalletHighload};

    #[near]
    impl WalletHighload for Contract {
        #[payable]
        fn wh_execute_signed(&mut self, msg: RequestMessage<TimeoutNonce>, proof: String) {
            self.execute_signed(msg, &proof)
                .unwrap_or_else(|err| utils::panic_msg(err));
        }
    }
};

type Result<T> = ::core::result::Result<T, Error<ContractNonceError<Contract>>>;

impl Contract {
    fn execute_signed(&mut self, msg: ContractRequestMessage<Self>, proof: &str) -> Result<()> {
        if !self.is_signature_allowed() {
            return Err(Error::SignatureDisabled);
        }

        // check chain_id
        if msg.chain_id != utils::chain_id() {
            return Err(Error::InvalidChainId {
                got: msg.chain_id,
                expected: utils::chain_id(),
            });
        }

        // check signer_id
        if msg.signer_id != env::current_account_id() {
            return Err(Error::InvalidSignerId(msg.signer_id));
        }

        // It makes sense to emit here, since checks above ensure this request
        // is intended to be relayed to this instance of wallet-contract.
        // In case following checks or request execution fail, this would
        // at least give relayers/indexers some observability on what exact
        // request hash was it.
        let hash = msg.hash();
        WalletEvent::SignedRequest { hash }.emit();

        // verify signature
        if !<Self as SigningStandardImpl>::SigningStandard::verify(&msg, &self.public_key, proof) {
            return Err(Error::InvalidSignature);
        }

        // commit nonce
        self.nonces.commit(msg.nonce).map_err(Error::Nonce)?;

        self.execute_request(msg.request, &Actor::SignedRequest(hash))
    }

    fn execute_extension(&mut self, request: Request) -> Result<()> {
        if env::attached_deposit().is_zero() {
            return Err(Error::InsufficientDeposit);
        }

        let extension_id = env::predecessor_account_id();
        self.check_extension_enabled(&extension_id)?;

        self.execute_request(request, &Actor::Extension(extension_id.into()))
    }

    fn execute_request(&mut self, request: Request, actor: &Actor<'_>) -> Result<()> {
        for op in request.ops {
            self.execute_op(op, actor.as_ref())?;
        }

        if let Some(promise) = request.out.build() {
            promise.detach();
        }

        Ok(())
    }

    fn execute_op(&mut self, op: WalletOp, actor: Actor<'_>) -> Result<()> {
        match op {
            WalletOp::SetSignatureMode { enable } => self.set_signature_mode(enable, actor),
            WalletOp::AddExtension { account_id } => self.add_extension(account_id, actor),
            WalletOp::RemoveExtension { account_id } => self.remove_extension(account_id, actor),
            // custom ops are not supported, so we just skip them
            // TODO: or panic?
            WalletOp::Custom { .. } => Ok(()),
        }
    }

    fn set_signature_mode(&mut self, enable: bool, actor: Actor<'_>) -> Result<()> {
        // emit first to help for debugging
        WalletEvent::SignatureModeSet {
            enabled: enable,
            by: actor,
        }
        .emit();

        if self.signature_enabled == enable {
            return Err(Error::ThisSignatureModeAlreadySet);
        }
        self.signature_enabled = enable;

        self.check_lockout()
    }

    fn add_extension(&mut self, account_id: AccountId, actor: Actor<'_>) -> Result<()> {
        // emit first to help for debugging
        WalletEvent::ExtensionAdded {
            account_id: (&account_id).into(),
            by: actor,
        }
        .emit();

        if !self.extensions.insert(account_id.clone()) {
            return Err(Error::ExtensionEnabled(account_id));
        }

        Ok(())
    }

    fn remove_extension(&mut self, account_id: AccountId, actor: Actor<'_>) -> Result<()> {
        // emit first to help for debugging
        WalletEvent::ExtensionRemoved {
            account_id: (&account_id).into(),
            by: actor,
        }
        .emit();

        if !self.extensions.remove(&account_id) {
            return Err(Error::ExtensionNotEnabled(account_id));
        }

        self.check_lockout()
    }

    fn check_extension_enabled(&self, account_id: &AccountIdRef) -> Result<()> {
        if !self.has_extension(account_id) {
            return Err(Error::ExtensionNotEnabled(account_id.to_owned()));
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
