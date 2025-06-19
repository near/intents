use defuse_crypto::{CryptoHash, Curve, Ed25519, Payload, SignedPayload, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;

/// See [SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md)
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone)]
#[must_use]
pub struct Sep53Payload {
    pub message: Vec<u8>,
}

impl Sep53Payload {
    #[inline]
    pub const fn new(message: Vec<u8>) -> Self {
        Self { message }
    }

    #[inline]
    pub fn prehash(&self) -> Vec<u8> {
        b"Stellar Signed Message:\n"
            .iter()
            .copied()
            .chain(self.message.as_slice().iter().copied())
            .collect()
    }
}

impl Payload for Sep53Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        env::sha256_array(&self.prehash())
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedSep53Payload {
    pub payload: Sep53Payload,

    #[serde_as(as = "AsCurve<Ed25519>")]
    pub public_key: <Ed25519 as Curve>::PublicKey,
    #[serde_as(as = "AsCurve<Ed25519>")]
    pub signature: <Ed25519 as Curve>::Signature,
}

impl Payload for SignedSep53Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedSep53Payload {
    type PublicKey = <Ed25519 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Ed25519::verify(&self.signature, &self.hash(), &self.public_key)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Sep53Payload, SignedSep53Payload};
    use base64::{Engine, engine::general_purpose::STANDARD};
    use defuse_crypto::{Payload, SignedPayload};
    use ed25519_dalek::Verifier;
    use ed25519_dalek::{SigningKey, ed25519::signature::SignerMut};
    use near_sdk::base64;
    use stellar_strkey::Strkey;

    #[test]
    fn reference_test_vectors() {
        // 1) Decode the StrKey seed -> raw 32 bytes
        let seed = "SAKICEVQLYWGSOJS4WW7HZJWAHZVEEBS527LHK5V4MLJALYKICQCJXMW";
        let raw = match Strkey::from_string(seed).unwrap() {
            Strkey::PrivateKeyEd25519(pk) => pk.0,
            _ => panic!("expected an Ed25519 seed"),
        };

        // 2) Build SigningKey + VerifyingKey
        let mut signing_key = SigningKey::from_bytes(&raw);
        let verifying_key = signing_key.verifying_key();

        let vectors = [
            (
                b"Hello, World!".as_ref(),
                "fO5dbYhXUhBMhe6kId/cuVq/AfEnHRHEvsP8vXh03M1uLpi5e46yO2Q8rEBzu3feXQewcQE5GArp88u6ePK6BA==",
            ),
            (
                "こんにちは、世界！".as_bytes(),
                "CDU265Xs8y3OWbB/56H9jPgUss5G9A0qFuTqH2zs2YDgTm+++dIfmAEceFqB7bhfN3am59lCtDXrCtwH2k1GBA==",
            ),
            (
                &STANDARD
                    .decode("2zZDP1sa1BVBfLP7TeeMk3sUbaxAkUhBhDiNdrksaFo=")
                    .unwrap(),
                "VA1+7hefNwv2NKScH6n+Sljj15kLAge+M2wE7fzFOf+L0MMbssA1mwfJZRyyrhBORQRle10X1Dxpx+UOI4EbDQ==",
            ),
        ];

        // Verify with dalek
        for (msg, expected_b64) in vectors {
            let mut payload = b"Stellar Signed Message:\n".to_vec();
            payload.extend_from_slice(msg);

            let hash = near_sdk::env::sha256_array(&payload);
            let sig = signing_key.sign(hash.as_ref());
            let actual_b64 = STANDARD.encode(sig.to_bytes());

            assert_eq!(actual_b64, *expected_b64);
            assert!(verifying_key.verify(hash.as_ref(), &sig).is_ok());
        }

        // Verify with our abstraction
        for (msg, expected_sig_b64) in vectors {
            let payload = Sep53Payload::new(msg.to_vec());

            let hash = payload.hash();
            let secret_key = near_crypto::SecretKey::ED25519(near_crypto::ED25519SecretKey(
                signing_key
                    .as_bytes()
                    .iter()
                    .chain(verifying_key.as_bytes())
                    .copied()
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            ));
            let generic_sig = secret_key.sign(hash.as_ref());
            let sig = match generic_sig {
                near_crypto::Signature::ED25519(signature) => signature,
                near_crypto::Signature::SECP256K1(_) => unreachable!(),
            };

            let actual_sig_b64 = STANDARD.encode(sig.to_bytes());

            assert_eq!(actual_sig_b64, *expected_sig_b64);
            assert!(generic_sig.verify(hash.as_ref(), &secret_key.public_key()));

            let signed_payload = SignedSep53Payload {
                payload,
                public_key: verifying_key.as_bytes().to_owned(),
                signature: sig.to_bytes(),
            };

            assert!(signed_payload.verify().is_some());
        }
    }
}
