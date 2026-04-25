use hkdf::Hkdf;

use crate::{DerivableSigningKey, ed25519::Ed25519, secp256k1::Secp256k1};

#[derive(Clone)]
pub struct InMemorySigner {
    ed25519_root_sk: crate::ed25519::SigningKey,
    secp256k1_root_sk: crate::secp256k1::SecretKey,
}

impl InMemorySigner {
    pub fn from_seed(seed: &[u8]) -> Self {
        // no salt is needed, seed is already with high entropy
        let hk = Hkdf::<sha3::Sha3_512>::new(None, seed);

        Self {
            ed25519_root_sk: {
                let mut sk = ed25519_dalek::SecretKey::default();
                hk.expand(
                    b"ed25519", // TODO
                    &mut sk,
                )
                .unwrap();
                ed25519_dalek::SigningKey::from_bytes(&sk)
            },
            secp256k1_root_sk: {
                // TODO: SHA3-256 would have done the job in one round, too
                let mut sk = [0u8; 32];
                hk.expand(
                    b"secp256k1", // TODO
                    &mut sk,
                )
                .unwrap();
                k256::SecretKey::from_bytes(&sk.into())
                    // TODO: handle zero
                    .unwrap()
            },
        }
    }
}

impl DerivableSigningKey<Ed25519> for InMemorySigner {
    type PublicKey = crate::ed25519::VerifyingKey;

    fn public_key(&self) -> Self::PublicKey {
        self.ed25519_root_sk.public_key()
    }

    fn sign_derive_from_tweak(
        &self,
        tweak: <Ed25519 as crate::DerivableCurve>::Tweak,
        msg: &[u8],
    ) -> <Ed25519 as crate::DerivableCurve>::Signature {
        self.ed25519_root_sk.sign_derive_from_tweak(tweak, msg)
    }
}

impl DerivableSigningKey<Secp256k1> for InMemorySigner {
    type PublicKey = crate::secp256k1::PublicKey;

    fn public_key(&self) -> Self::PublicKey {
        self.secp256k1_root_sk.public_key()
    }

    fn sign_derive_from_tweak(
        &self,
        tweak: <Secp256k1 as crate::DerivableCurve>::Tweak,
        msg: &[u8],
    ) -> <Secp256k1 as crate::DerivableCurve>::Signature {
        self.secp256k1_root_sk.sign_derive_from_tweak(tweak, msg)
    }
}
