use defuse_crypto::{CryptoHash, Curve, Ed25519, Payload, SignedPayload, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;

/// See [SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md)
#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub struct Sep53Payload {
    pub payload: String,
}

impl Sep53Payload {
    #[inline]
    pub const fn new(payload: String) -> Self {
        Self { payload }
    }

    #[inline]
    pub fn prehash(&self) -> Vec<u8> {
        [b"Stellar Signed Message:\n", self.payload.as_bytes()].concat()
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
    #[serde(flatten)]
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
    use defuse_test_utils::random::{CryptoRng, Rng, gen_random_string, random_bytes, rng};
    use ed25519_dalek::Verifier;
    use ed25519_dalek::{SigningKey, ed25519::signature::SignerMut};
    use near_sdk::base64;
    use rstest::rstest;
    use stellar_strkey::Strkey;

    #[test]
    fn reference_test_vectors() {
        // 1) Decode the StrKey seed -> raw 32 bytes
        let seed = "SAKICEVQLYWGSOJS4WW7HZJWAHZVEEBS527LHK5V4MLJALYKICQCJXMW";
        let raw_key = match Strkey::from_string(seed).unwrap() {
            Strkey::PrivateKeyEd25519(pk) => pk.0,
            _ => panic!("expected an Ed25519 seed"),
        };

        // 2) Build SigningKey + VerifyingKey
        let mut signing_key = SigningKey::from_bytes(&raw_key);
        let verifying_key = signing_key.verifying_key();

        let vectors = [
            (
                "Hello, World!",
                "fO5dbYhXUhBMhe6kId/cuVq/AfEnHRHEvsP8vXh03M1uLpi5e46yO2Q8rEBzu3feXQewcQE5GArp88u6ePK6BA==",
            ),
            (
                "こんにちは、世界！",
                "CDU265Xs8y3OWbB/56H9jPgUss5G9A0qFuTqH2zs2YDgTm+++dIfmAEceFqB7bhfN3am59lCtDXrCtwH2k1GBA==",
            ),
            // One test vector is dropped because it's binary data, and that's not supported
        ];

        // Verify with dalek
        for (msg, expected_b64) in vectors {
            let mut payload = "Stellar Signed Message:\n".to_string();
            payload += msg;

            let hash = near_sdk::env::sha256_array(payload.as_bytes());
            let sig = signing_key.sign(hash.as_ref());
            let actual_b64 = STANDARD.encode(sig.to_bytes());

            assert_eq!(actual_b64, *expected_b64);
            assert!(verifying_key.verify(hash.as_ref(), &sig).is_ok());
        }

        // Verify with our abstraction
        for (msg, expected_sig_b64) in vectors {
            let payload = Sep53Payload::new(msg.to_string());

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

            assert_eq!(
                signed_payload.verify(),
                Some(verifying_key.as_bytes().to_owned())
            );
        }
    }

    /// Returns a new String where one character in `s` is replaced by a random lowercase ASCII letter.
    fn tamper_string(rng: &mut impl Rng, s: &str) -> String {
        let mut chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        if len == 0 {
            return String::new();
        }

        let idx = rng.random_range(0..len);
        // keep sampling until we get a new char
        let new_c = loop {
            #[allow(clippy::as_conversions)]
            let c = (b'a' + rng.random_range(0..26)) as char;
            if c != chars[idx] {
                break c;
            }
        };
        chars[idx] = new_c;
        chars.into_iter().collect()
    }

    /// Returns a new signature byte‐vector where exactly one bit of the original `sig`
    /// has been flipped at a random position.
    fn tamper_bytes(rng: &mut impl Rng, sig: &[u8]) -> Vec<u8> {
        let mut tampered = sig.to_vec();
        let total_bits = tampered.len() * 8;
        // pick a random bit index and flip it
        let bit_idx = rng.random_range(0..total_bits);
        let byte_idx = bit_idx / 8;
        let bit_in_byte = bit_idx % 8;
        tampered[byte_idx] ^= 1 << bit_in_byte;
        tampered
    }

    /// Decode our test seed into a NEAR ED25519 secret + public key
    fn make_ed25519_key(rng: &mut (impl Rng + CryptoRng)) -> near_crypto::SecretKey {
        // We have to use dalek because near interface doesn't support making keys from bytes
        // so we start from dalek, generate a random key, then use it in a new near_crypto key
        let key_len = ed25519_dalek::SECRET_KEY_LENGTH;
        let bytes = random_bytes(key_len..=key_len, rng);
        let signing_key = SigningKey::from_bytes(&bytes.try_into().unwrap());
        let verifying_key = signing_key.verifying_key();

        near_crypto::SecretKey::ED25519(near_crypto::ED25519SecretKey(
            signing_key
                .as_bytes()
                .iter()
                .chain(verifying_key.as_bytes())
                .copied()
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        ))
    }

    #[rstest]
    fn tampered_message_fails(mut rng: impl Rng + CryptoRng) {
        let sk = make_ed25519_key(&mut rng);
        let pk = sk.public_key();

        let msg = gen_random_string(&mut rng, 100..1000);

        // sign the “good” message
        let payload = Sep53Payload::new(msg.clone());
        let hash = payload.hash();
        let sig = match sk.sign(hash.as_ref()) {
            near_crypto::Signature::ED25519(signature) => signature,
            near_crypto::Signature::SECP256K1(_) => unreachable!(),
        };

        {
            let signed_good = SignedSep53Payload {
                payload,
                public_key: pk.key_data().try_into().unwrap(),
                signature: sig.to_bytes(),
            };
            assert!(signed_good.verify().is_some());
        }

        // tamper with the message, and expect failure
        {
            let tempered_message = tamper_string(&mut rng, &msg);

            // verify with a tampered message
            let bad_payload = Sep53Payload::new(tempered_message);
            let signed_bad = SignedSep53Payload {
                payload: bad_payload,
                public_key: pk.key_data().try_into().unwrap(),
                signature: sig.to_bytes(),
            };
            assert_eq!(signed_bad.verify(), None);
        }
    }

    #[rstest]
    fn tampered_signature_fails(mut rng: impl Rng + CryptoRng) {
        let sk = make_ed25519_key(&mut rng);
        let pk = sk.public_key();

        let msg = gen_random_string(&mut rng, 100..1000);

        // sign the canonical payload
        let payload = Sep53Payload::new(msg);
        let hash = payload.hash();
        let sig = match sk.sign(hash.as_ref()) {
            near_crypto::Signature::ED25519(signature) => signature,
            near_crypto::Signature::SECP256K1(_) => unreachable!(),
        };

        {
            let signed_good = SignedSep53Payload {
                payload: payload.clone(),
                public_key: pk.key_data().try_into().unwrap(),
                signature: sig.into(),
            };
            assert!(signed_good.verify().is_some());
        }

        // tamper with the signature, and expect failure
        {
            let bad_bytes = tamper_bytes(&mut rng, sig.to_bytes().as_ref());

            let signed_bad = SignedSep53Payload {
                payload,
                public_key: pk.key_data().try_into().unwrap(),
                signature: bad_bytes.try_into().unwrap(),
            };
            assert!(signed_bad.verify().is_none());
        }
    }
}
