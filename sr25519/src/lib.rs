//! Sr25519 message signing support for Polkadot/Substrate chains
//!
//! This crate implements message signing for Polkadot/Substrate-based chains
//! using the Sr25519 (Schnorr on Ristretto255) signature scheme.
//!
//! Compatible with Polkadot.js, Talisman, Subwallet, and other Substrate wallets.

use defuse_crypto::{Curve, Payload, SignedPayload, Sr25519, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;

/// Raw message payload for Sr25519 signing
///
/// Uses the "substrate" signing context following Substrate/Polkadot conventions.
/// This matches how Polkadot.js and other Substrate wallets sign messages.
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Sr25519Payload {
    /// The message to be signed
    pub payload: String,
}

impl Sr25519Payload {
    /// Create a new payload with a message.
    ///
    /// NOTE: This will use the standard "substrate" signing context
    /// which is the default for Polkadot/Substrate chains.
    #[inline]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            payload: message.into(),
        }
    }
}

impl Payload for Sr25519Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        // Hash just the message content. The signing context is handled
        // by schnorrkel during signature verification.
        //
        // SHA-256 is chosen because it's one of two cryptographic hash
        // functions commonly used in Polkadot/Substrate ecosystem. The
        // second one is Blake2, which is not natively supported on Near
        env::sha256_array(&self.payload)
    }
}

/// Signed Sr25519 message with signature
#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedSr25519Payload {
    #[serde(flatten)]
    pub payload: Sr25519Payload,

    /// sr25519 public key: Ristretto Schnorr public key
    /// represented as a 32-byte Ristretto compressed point
    #[serde_as(as = "AsCurve<Sr25519>")]
    pub public_key: <Sr25519 as Curve>::PublicKey,

    /// sr25519 signature: 64-byte Ristretto Schnorr signature
    #[serde_as(as = "AsCurve<Sr25519>")]
    pub signature: <Sr25519 as Curve>::Signature,
}

impl Payload for SignedSr25519Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedSr25519Payload {
    type PublicKey = <Sr25519 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        // For Sr25519, we need to pass the raw message bytes (not formatted)
        // The Sr25519::verify implementation will apply the "substrate" context
        // However, since schnorrkel's sign_simple and verify_simple both use
        // the same context parameter, we just pass the message directly
        Sr25519::verify(
            &self.signature,
            self.payload.payload.as_bytes(),
            &self.public_key,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_creation() {
        let payload = Sr25519Payload::new("Hello, Sr25519!".to_string());
        assert_eq!(payload.payload, "Hello, Sr25519!");
    }

    #[test]
    fn test_payload_hashing() {
        let payload = Sr25519Payload::new("test message".to_string());
        let hash = payload.hash();
        // Verify hash is 32 bytes
        assert_eq!(hash.len(), 32);

        // Same message should produce same hash
        let payload2 = Sr25519Payload::new("test message".to_string());
        assert_eq!(payload.hash(), payload2.hash());
    }
}
