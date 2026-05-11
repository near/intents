//! Sr25519 message signing for Polkadot/Substrate chains.
//!
//! Implements message signing for Polkadot/Substrate-based chains using the
//! Sr25519 (Schnorr on Ristretto255) signature scheme. Compatible with
//! Polkadot.js, Talisman, Subwallet, and other Substrate wallets.
//!
//! Substrate wallets wrap arbitrary messages in `<Bytes>...</Bytes>` tags
//! before signing. This crate replicates that wrapping during verification,
//! so the [`Sr25519Payload`] carries the *unwrapped* user-facing message.

use defuse_crypto::{CryptoHash, Curve, Payload, SignedPayload, Sr25519, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near, serde_with::serde_as};

/// Raw message payload for Sr25519 signing.
///
/// The `payload` field holds the **inner** message content (without the
/// `<Bytes>` wrapper that Substrate wallets apply automatically at sign time).
#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub struct Sr25519Payload {
    pub payload: String,
}

impl Sr25519Payload {
    #[inline]
    pub const fn new(payload: String) -> Self {
        Self { payload }
    }

    /// The exact bytes the signer puts through schnorrkel — i.e. the inner
    /// message wrapped in `<Bytes>...</Bytes>` exactly like Polkadot.js does.
    #[inline]
    pub fn signed_message(&self) -> Vec<u8> {
        [b"<Bytes>", self.payload.as_bytes(), b"</Bytes>"].concat()
    }
}

impl Payload for Sr25519Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        // SHA-256 is one of the two hash functions commonly used in the
        // Polkadot/Substrate ecosystem (the other is Blake2, which has no
        // native host function on NEAR).
        //
        // This hash is used by callers for replay protection / nonce keying;
        // schnorrkel does its own internal hashing via merlin during verify.
        env::sha256_array(self.signed_message())
    }
}

#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedSr25519Payload {
    #[serde(flatten)]
    pub payload: Sr25519Payload,

    /// 32-byte Ristretto Schnorr public key.
    #[serde_as(as = "AsCurve<Sr25519>")]
    pub public_key: <Sr25519 as Curve>::PublicKey,

    /// 64-byte Ristretto Schnorr signature.
    #[serde_as(as = "AsCurve<Sr25519>")]
    pub signature: <Sr25519 as Curve>::Signature,
}

impl Payload for SignedSr25519Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedSr25519Payload {
    type PublicKey = <Sr25519 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Sr25519::verify(
            &self.signature,
            &self.payload.signed_message(),
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
    fn signed_message_is_bytes_wrapped() {
        let p = Sr25519Payload::new("Hello".to_string());
        assert_eq!(p.signed_message(), b"<Bytes>Hello</Bytes>");
    }

    #[test]
    fn hash_is_deterministic() {
        let a = Sr25519Payload::new("test".into());
        let b = Sr25519Payload::new("test".into());
        assert_eq!(a.hash(), b.hash());

        let c = Sr25519Payload::new("different".into());
        assert_ne!(a.hash(), c.hash());
    }

    #[test]
    fn roundtrip_signature() {
        let keypair = Keypair::generate_with(&mut thread_rng());
        let payload = Sr25519Payload::new("Hello, Sr25519!".into());

        let signature = keypair.sign_simple(Sr25519::SIGNING_CTX, &payload.signed_message());

        let signed = SignedSr25519Payload {
            payload,
            public_key: keypair.public.to_bytes(),
            signature: signature.to_bytes(),
        };

        assert_eq!(signed.verify(), Some(keypair.public.to_bytes()));
    }

    #[test]
    fn invalid_signature_rejected() {
        let keypair = Keypair::generate_with(&mut thread_rng());
        let signed = SignedSr25519Payload {
            payload: Sr25519Payload::new("x".into()),
            public_key: keypair.public.to_bytes(),
            signature: [0u8; 64],
        };
        assert!(signed.verify().is_none());
    }

    #[test]
    fn cross_message_signature_rejected() {
        let keypair = Keypair::generate_with(&mut thread_rng());
        let p1 = Sr25519Payload::new("first".into());
        let p2 = Sr25519Payload::new("second".into());
        let sig1 = keypair.sign_simple(Sr25519::SIGNING_CTX, &p1.signed_message());

        let signed = SignedSr25519Payload {
            payload: p2,
            public_key: keypair.public.to_bytes(),
            signature: sig1.to_bytes(),
        };
        assert!(signed.verify().is_none());
    }

    /// Real signature produced by Polkadot.js Extension on 2025-12-18, kept
    /// as a regression vector so the `<Bytes>` wrapping stays correct.
    ///
    /// - Address: `167y8dsUr7kaM1FNoCtXWy2unEnjGHiN7ML3vawR6Nwywbci`
    /// - Message: `"Hello from Intents!"`
    /// - Signed via <https://polkadot.js.org/apps/#/signing>
    #[test]
    fn real_polkadot_js_signature() {
        let public_key = hex!("e27d987db9ed2a7a48f4137c997d610226dc93bf256c9026268b0b8489bb9862");
        let signature = hex!(
            "e2c01abbd53c89d6302475827b62c7e2168a93a407ebafd94fee3fb2e286e539"
            "ee1877c15df48c55c59f9d5e032f1f9a1b63a2dc4085517d705ec174e6c9cf8c"
        );

        let signed = SignedSr25519Payload {
            payload: Sr25519Payload::new("Hello from Intents!".into()),
            public_key,
            signature,
        };

        assert_eq!(signed.verify(), Some(public_key));
    }
}
