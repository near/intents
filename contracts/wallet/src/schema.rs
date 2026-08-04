use defuse_nep641::OffchainMessage;

use crate::RequestMessage;

/// Signature schema used by [`Wallet`](crate::contract::Wallet) contract
/// variant.
///
/// By design, each wallet contract variant implements its own schema and
/// gets deployed separately.
pub trait SignatureSchema {
    /// Public key used by this schema and [stored](field@crate::State::public_key)
    /// in the contract's state.
    ///
    ///
    /// Its [`Display`](core::fmt::Display) implementation is returned from
    /// [`w_public_key()`](crate::contract::Wallet::w_public_key) contract
    /// method.
    type PublicKey;

    /// Verify given proof over the request message in respect to the public key
    /// and return whether verification passed.
    ///
    /// Used by the [`w_execute_signed()`](crate::contract::Wallet::w_execute_signed) contract method.
    #[must_use = "check if verification passed"]
    fn verify_request_msg(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool;

    /// Verify given proof over the offchain message in respect to the public key
    /// and return whether verification passed.
    ///
    /// Used by the [`w_resolve_auth()`](defuse_nep641::AuthResolver::w_resolve_auth) contract method.
    #[must_use = "check if verification passed"]
    fn verify_offchain_msg(
        public_key: &Self::PublicKey,
        msg: &OffchainMessage,
        proof: &str,
    ) -> bool;
}
