//! Traits for hashing and verifying message payloads used within Intents.
//!
//! These traits provide a small abstraction layer over various signing
//! standards supported by Intents. Each standard (e.g. BIP-322, TIP-191,
//! ERC-191) defines its own structure that implements [`Payload`] and
//! [`SignedPayload`]. The implementations expose a uniform API so the
//! Intents engine can compute message hashes and verify signatures without
//! knowing the concrete standard.

pub use near_sdk::CryptoHash;

/// Data that can be deterministically hashed for signing or verification.
///
/// Implementations of this trait typically represent a message formatted
/// according to an external signing standard. The [`hash`] method returns
/// the digest that should be signed or used for verification.
pub trait Payload {
    fn hash(&self) -> CryptoHash;
}

/// Extension of [`Payload`] for types that include a signature.
///
/// Implementers verify the signature and, when successful, return the
/// signer's public key. This trait is mainly intended for internal use and
/// does not constitute a stable public API.
pub trait SignedPayload: Payload {
    type PublicKey;

    fn verify(&self) -> Option<Self::PublicKey>;
}
