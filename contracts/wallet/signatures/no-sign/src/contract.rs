use defuse_wallet::{State, contract::Error, wallet};
use near_sdk::{FunctionError, env, near};

use crate::{NoPublicKey, NoSign};

wallet! {
    #[wallet(
        schema = NoSign,
        metadata(
            standard(standard = "wallet-no-sign", version = "1.0.0")
        )
    )]
    /// Wallet Contract variant which always rejects the signature.
    struct Contract(_);
}

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
    #[cfg_attr(not(near), allow(dead_code))]
    #[allow(clippy::use_self)]
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
