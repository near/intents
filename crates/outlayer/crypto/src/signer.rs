use hkdf::Hkdf;
use sha3::Sha3_512;
use zeroize::ZeroizeOnDrop;

use crate::{DeriveSigner, ed25519::Ed25519, secp256k1::Secp256k1};

#[derive(Clone, ZeroizeOnDrop)]
pub struct InMemorySigner {
    ed25519_root_sk: crate::ed25519::SigningKey,
    secp256k1_root_sk: crate::secp256k1::SecretKey,
}

impl InMemorySigner {
    const SALT: &'static [u8] = b"outlayer v0.1.0 ikm seed:";

    pub fn from_seed(seed: &[u8]) -> Self {
        // no salt is needed, seed is already with high entropy
        let hk = Hkdf::<Sha3_512>::new(Some(Self::SALT), seed);

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

impl DeriveSigner<Ed25519> for InMemorySigner {
    type PublicKey = crate::ed25519::VerifyingKey;

    fn public_key(&self) -> Self::PublicKey {
        self.ed25519_root_sk.public_key()
    }

    fn sign(
        &self,
        tweak: <Ed25519 as crate::DerivableCurve>::Tweak,
        msg: &[u8],
    ) -> <Ed25519 as crate::DerivableCurve>::Signature {
        self.ed25519_root_sk.sign(tweak, msg)
    }
}

impl DeriveSigner<Secp256k1> for InMemorySigner {
    type PublicKey = crate::secp256k1::PublicKey;

    fn public_key(&self) -> Self::PublicKey {
        self.secp256k1_root_sk.public_key()
    }

    fn sign(
        &self,
        tweak: <Secp256k1 as crate::DerivableCurve>::Tweak,
        msg: &[u8],
    ) -> <Secp256k1 as crate::DerivableCurve>::Signature {
        self.secp256k1_root_sk.sign(tweak, msg)
    }
}
