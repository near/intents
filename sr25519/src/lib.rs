//! Sr25519 message signing support for Polkadot/Substrate chains
//!
//! This crate implements message signing for Polkadot/Substrate-based chains
//! using the Sr25519 (Schnorr on Ristretto255) signature scheme.
//!
//! Compatible with Polkadot.js, Talisman, Subwallet, and other Substrate wallets.

use defuse_crypto::{Curve, Payload, SignedPayload, Sr25519, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near};

/// Raw message payload for Sr25519 signing
///
/// Uses the "substrate" signing context following Substrate/Polkadot conventions.
/// This matches how Polkadot.js and other Substrate wallets sign messages.
///
/// Note: Substrate wallets (Polkadot.js, Talisman, Subwallet, etc.) wrap messages
/// in `<Bytes>...</Bytes>` tags when signing. This wrapping is handled automatically
/// during verification, so the payload should contain only the inner message content.
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Sr25519Payload {
    /// The message to be signed (without `<Bytes>` wrapper)
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
        // Substrate wallets (Polkadot.js, Talisman, Subwallet, etc.) wrap messages
        // in <Bytes>...</Bytes> tags when signing. We need to apply the same wrapping
        // during verification to match what was actually signed by the wallet.
        let wrapped_message = format!("<Bytes>{}</Bytes>", self.payload.payload);

        Sr25519::verify(
            &self.signature,
            wrapped_message.as_bytes(),
            &self.public_key,
        )
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rand::thread_rng;
    use schnorrkel::Keypair;

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

    #[test]
    fn test_sr25519_signature_verification() {
        // Generate a keypair
        let mut rng = thread_rng();
        let keypair = Keypair::generate_with(&mut rng);

        // Create a message
        let message = "Hello, Sr25519!";

        // Sign the message with <Bytes> wrapper (matching wallet behavior)
        let wrapped_message = format!("<Bytes>{}</Bytes>", message);
        let signature = keypair.sign_simple(Sr25519::SIGNING_CTX, wrapped_message.as_bytes());

        // Create the signed payload with unwrapped message
        let signed_payload = SignedSr25519Payload {
            payload: Sr25519Payload::new(message.to_string()),
            public_key: keypair.public.to_bytes(),
            signature: signature.to_bytes(),
        };

        // Verify the signature
        let verified_key = signed_payload.verify();
        assert!(verified_key.is_some(), "Signature verification failed");
        assert_eq!(
            verified_key.unwrap(),
            keypair.public.to_bytes(),
            "Recovered public key doesn't match"
        );
    }

    #[test]
    fn test_sr25519_invalid_signature() {
        // Generate a keypair
        let mut rng = thread_rng();
        let keypair = Keypair::generate_with(&mut rng);

        // Create a message
        let message = "Hello, Sr25519!";

        // Create an invalid signature (all zeros)
        let invalid_signature = [0u8; 64];

        // Create the signed payload with invalid signature
        let signed_payload = SignedSr25519Payload {
            payload: Sr25519Payload::new(message.to_string()),
            public_key: keypair.public.to_bytes(),
            signature: invalid_signature,
        };

        // Verify should fail
        let verified_key = signed_payload.verify();
        assert!(
            verified_key.is_none(),
            "Invalid signature was incorrectly verified"
        );
    }

    #[test]
    fn test_sr25519_different_messages() {
        // Generate a keypair
        let mut rng = thread_rng();
        let keypair = Keypair::generate_with(&mut rng);

        // Sign two different messages with <Bytes> wrapper (matching wallet behavior)
        let message1 = "First message";
        let message2 = "Second message";

        let wrapped1 = format!("<Bytes>{}</Bytes>", message1);
        let wrapped2 = format!("<Bytes>{}</Bytes>", message2);

        let signature1 = keypair.sign_simple(Sr25519::SIGNING_CTX, wrapped1.as_bytes());
        let signature2 = keypair.sign_simple(Sr25519::SIGNING_CTX, wrapped2.as_bytes());

        // Verify first signature
        let signed_payload1 = SignedSr25519Payload {
            payload: Sr25519Payload::new(message1.to_string()),
            public_key: keypair.public.to_bytes(),
            signature: signature1.to_bytes(),
        };
        assert!(
            signed_payload1.verify().is_some(),
            "First signature verification failed"
        );

        // Verify second signature
        let signed_payload2 = SignedSr25519Payload {
            payload: Sr25519Payload::new(message2.to_string()),
            public_key: keypair.public.to_bytes(),
            signature: signature2.to_bytes(),
        };
        assert!(
            signed_payload2.verify().is_some(),
            "Second signature verification failed"
        );

        // Cross-verification should fail (signature1 with message2)
        let cross_payload = SignedSr25519Payload {
            payload: Sr25519Payload::new(message2.to_string()),
            public_key: keypair.public.to_bytes(),
            signature: signature1.to_bytes(),
        };
        assert!(
            cross_payload.verify().is_none(),
            "Cross-verification should fail"
        );
    }

    #[test]
    fn test_payload_hash_consistency() {
        let payload1 = Sr25519Payload::new("test".to_string());
        let payload2 = Sr25519Payload::new("test".to_string());

        // Same payload should produce same hash
        assert_eq!(payload1.hash(), payload2.hash());

        let payload3 = Sr25519Payload::new("different".to_string());
        // Different payload should produce different hash
        assert_ne!(payload1.hash(), payload3.hash());
    }

    #[test]
    fn test_real_wallet_signature() {
        // Real signature from Polkadot.js Extension
        // Wallet: Polkadot.js Extension (https://polkadot.js.org/extension/)
        // Test dApp: https://polkadot.js.org/apps/#/signing
        // Date: 2025-12-18
        // Address: 167y8dsUr7kaM1FNoCtXWy2unEnjGHiN7ML3vawR6Nwywbci
        // Original message: "Hello from Intents!"
        // Note: Polkadot.js wraps messages in <Bytes></Bytes> tags when signing.
        // The wrapping is handled automatically during verification.

        let message = "Hello from Intents!";

        // Public key (32 bytes)
        let public_key = hex!("e27d987db9ed2a7a48f4137c997d610226dc93bf256c9026268b0b8489bb9862");

        // Signature (64 bytes) signed via Polkadot.js
        let signature = hex!(
            "e2c01abbd53c89d6302475827b62c7e2168a93a407ebafd94fee3fb2e286e539
         ee1877c15df48c55c59f9d5e032f1f9a1b63a2dc4085517d705ec174e6c9cf8c"
        );

        // Create the signed payload with the message
        let signed_payload = SignedSr25519Payload {
            payload: Sr25519Payload::new(message.to_string()),
            public_key,
            signature,
        };

        // Verify the signature
        let verified_key = signed_payload.verify();
        assert_eq!(
            verified_key,
            Some(public_key),
            "Signature verification failed or Recovered public key doesn't match"
        );
    }
}
