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
    pub message: String,
}

impl Sr25519Payload {
    /// Create a new payload with a message
    /// 
    /// Note: This will use the standard "substrate" signing context
    /// which is the default for Polkadot/Substrate chains.
    #[inline]
    pub fn new(message: String) -> Self {
        Self { message }
    }

    /// Get the signing context (always "substrate")
    #[inline]
    pub const fn get_context(&self) -> &'static [u8] {
        b"substrate"
    }
}

impl Payload for Sr25519Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        // Hash just the message content
        // The signing context is handled by schnorrkel during signature verification
        env::sha256_array(self.message.as_bytes())
    }
}

/// Signed Sr25519 message with signature
#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedSr25519Payload {
    #[serde(flatten)]
    pub payload: Sr25519Payload,

    /// Sr25519 public key (32 bytes)
    #[serde_as(as = "AsCurve<Sr25519>")]
    pub public_key: <Sr25519 as Curve>::PublicKey,
    
    /// Sr25519 signature (64 bytes)
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
            self.payload.message.as_bytes(),
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
        assert_eq!(payload.message, "Hello, Sr25519!");
        assert_eq!(payload.get_context(), b"substrate");
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

    // Note: Real Sr25519 signature verification tests would require
    // generating valid signatures with schnorrkel, which is complex.
    // For production use, you should:
    // 1. Generate a keypair using schnorrkel
    // 2. Sign a message with the correct context
    // 3. Verify the signature here
    //
    // Example structure (would need schnorrkel in dev-dependencies):
    // #[test]
    // fn test_sr25519_verification() {
    //     use schnorrkel::{Keypair, signing_context};
    //     use rand::thread_rng;
    //     
    //     let keypair = Keypair::generate_with(thread_rng());
    //     let context = signing_context(b"substrate");
    //     let message = b"test";
    //     let signature = keypair.sign_simple(&context, message);
    //     
    //     let signed_payload = SignedHydrationPayload {
    //         payload: HydrationPayload::new("test".to_string()),
    //         public_key: keypair.public.to_bytes(),
    //         signature: signature.to_bytes(),
    //     };
    //     
    //     assert!(signed_payload.verify().is_some());
    // }
}
