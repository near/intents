use std::collections::BTreeSet;

use defuse_wallet_webauthn::{
    WalletWebauthn,
    core::{
        Request, RequestMessage, STATE_KEY, Timestamp,
        contract::{ContractImpl, Wallet},
    },
    p256::P256,
    webauthn::IgnoreUserVerification,
};
use near_sdk::{AccountId, PanicOnDefault, near};

#[cfg_attr(not(near), allow(dead_code))]
#[near(
    contract_state(key = STATE_KEY),
    contract_metadata(
        standard(standard = "wallet", version = "1.0.0"),
        standard(standard = "wallet-webauthn-p256", version = "1.0.0"),
    ),
)]
#[derive(Debug, PanicOnDefault)]
#[repr(transparent)]
// TODO: ignore user verification?
struct Contract(ContractImpl<WalletWebauthn<P256, IgnoreUserVerification>>);

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
