use std::collections::BTreeSet;

use defuse_wallet_core::{
    Request, RequestMessage, STATE_KEY, State, Timestamp,
    contract::{ContractImpl, Error, Wallet},
};
use near_sdk::{AccountId, FunctionError, PanicOnDefault, env, near};

use crate::{NoPublicKey, NoSign};

#[near(
    contract_state(key = STATE_KEY),
    contract_metadata(
        standard(standard = "wallet", version = "1.0.0"),
        standard(standard = "wallet-ed25519", version = "1.0.0"),
    ),
)]
#[derive(Debug, PanicOnDefault)]
#[repr(transparent)]
struct Contract(ContractImpl<NoSign>);

#[near]
impl Contract {
    /// Initialize a wallet contract on the existing account
    /// with authentication by signature disabled and
    /// add the current account as an extension.
    ///
    /// This method is allowed to be called only by the current
    /// account itself. It's recommended to call this method
    /// in the same receipt right after `UseGlobalContract` action.
    ///
    /// MUST attach at least 1yN for security reasons.
    #[private]
    #[payable]
    #[init]
    pub fn w_init() -> Self {
        if env::attached_deposit().is_zero() {
            // reject FunctionCall access keys
            Error::InsufficientDeposit.panic();
        }

        let mut s = State::new(NoPublicKey)
            // Add self as the only extension
            .extensions([env::current_account_id()]);

        // Disable signature verification completely,
        // so that accidently removing self from extensions
        // would result into lockout error.
        s.signature_enabled = false;

        Self(s.into())
    }
}

#[near]
impl Wallet for Contract {
    #[doc = " Execute signed request message."]
    #[doc = ""]
    #[doc = " SHOULD accept ANY attached deposit."]
    #[doc = ""]
    #[doc = " MUST fail in case where the `msg.request` was not executed"]
    #[doc = " due to various reasons, including:"]
    #[doc = "   * `msg` data is invalid"]
    #[doc = "   * `proof` is invalid"]
    #[doc = "   * signature is disabled"]
    #[doc = "   * nonce is already used"]
    #[payable]
    fn w_execute_signed(&mut self, msg: RequestMessage, proof: String) {
        self.0.w_execute_signed(msg, proof);
    }

    #[doc = " Execute request from an enabled extension."]
    #[doc = ""]
    #[doc = " * SHOULD accept ANY **non-zero** attached deposit"]
    #[doc = " * MUST panic if zero deposit was attached"]
    #[doc = " * MUST panic if [`predecessor_account_id`](near_sdk::env::predecessor_account_id)"]
    #[doc = "   extension is not enabled"]
    #[payable]
    fn w_execute_extension(&mut self, request: Request) {
        self.0.w_execute_extension(request);
    }

    #[doc = " Returns `subwallet_id`."]
    fn w_subwallet_id(&self) -> u32 {
        self.0.w_subwallet_id()
    }

    #[doc = " Returns whether authentication by signature is currently allowed."]
    fn w_is_signature_allowed(&self) -> bool {
        self.0.w_is_signature_allowed()
    }

    #[doc = " Returns a string representation of the public key or authentication"]
    #[doc = " identity associated with this wallet\'s singing standard."]
    fn w_public_key(&self) -> String {
        self.0.w_public_key()
    }

    #[doc = " Returns whether extension with given `account_id` is enabled."]
    #[doc = " If true, this `account_id` SHOULD be allowed to call"]
    #[doc = " `w_execute_extension()`."]
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool {
        self.0.w_is_extension_enabled(account_id)
    }

    #[doc = " Returns a set of enabled extensions. Each returned account"]
    #[doc = " SHOULD be allowed to call `w_execute_extension()`."]
    fn w_extensions(&self) -> BTreeSet<AccountId> {
        self.0.w_extensions()
    }

    #[doc = " Returns a timeout, i.e. validity timespan for each nonce."]
    fn w_timeout_secs(&self) -> u32 {
        self.0.w_timeout_secs()
    }

    #[doc = " Returns a timestamp when nonces were last cleaned up."]
    fn w_last_cleaned_at(&self) -> Timestamp {
        self.0.w_last_cleaned_at()
    }
}
