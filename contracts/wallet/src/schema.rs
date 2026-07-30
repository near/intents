use crate::{AuthMessage, RequestMessage};

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

    /// Verify given proof over a 32-byte domain-separated digest in respect
    /// to the public key and return whether verification passed.
    ///
    /// The digest is either a [canonical request hash](RequestMessage::hash)
    /// (for `w_execute_signed(msg, proof)`) or a
    /// [canonical authorization hash](crate::AuthMessage::hash)
    /// (for `w_resolve_auth(purpose, recipient, authorization)`).
    #[must_use = "check if verification passed"]
    fn verify_hash(public_key: &Self::PublicKey, hash: &[u8; 32], proof: &str) -> bool;

    /// Verify given proof over the request message in respect to the
    /// public key and return whether verification passed.
    ///
    /// Used by the `w_execute_signed(msg, proof)` contract method.
    ///
    /// Schemas that bind the proof to the *contents* of the message rather
    /// than to its digest (e.g. clear-signing standards, which show the
    /// signer what they are authorizing) MUST override this method.
    #[cfg(all(feature = "digest", feature = "borsh"))]
    #[must_use = "check if verification passed"]
    #[inline]
    fn verify(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
        Self::verify_hash(public_key, &msg.hash(), proof)
    }

    /// Verify given proof over the authorization message in respect to the
    /// public key and return whether verification passed.
    ///
    /// Used by the `w_resolve_auth(purpose, recipient, authorization)`
    /// contract method.
    ///
    /// Schemas that bind the proof to the *contents* of the message rather
    /// than to its digest (e.g. clear-signing standards, which show the
    /// signer what they are authorizing) MUST override this method.
    #[cfg(all(feature = "digest", feature = "borsh"))]
    #[must_use = "check if verification passed"]
    #[inline]
    fn verify_auth(public_key: &Self::PublicKey, msg: &AuthMessage, proof: &str) -> bool {
        Self::verify_hash(public_key, &msg.hash(), proof)
    }
}
