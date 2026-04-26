#![cfg(any(feature = "ed25519", feature = "secp256k1"))]

use hkdf::Hkdf;
use sha3::Sha3_512;
use zeroize::ZeroizeOnDrop;

use crate::{DerivableCurve, DeriveSigner};

#[derive(Clone, ZeroizeOnDrop)]
pub struct InMemorySigner {
    #[cfg(feature = "ed25519")]
    ed25519_root_sk: crate::ed25519::SigningKey,
    #[cfg(feature = "secp256k1")]
    secp256k1_root_sk: crate::secp256k1::SecretKey,
}

impl InMemorySigner {
    const SALT: &'static [u8] = b"outlayer v0.1.0 ikm seed:";

    pub fn from_seed(seed: &[u8]) -> Self {
        // no salt is needed, seed is already with high entropy
        let hk = Hkdf::<Sha3_512>::new(Some(Self::SALT), seed);

        Self {
            #[cfg(feature = "ed25519")]
            ed25519_root_sk: {
                let mut sk = ed25519_dalek::SecretKey::default();
                hk.expand(
                    b"ed25519", // TODO
                    &mut sk,
                )
                .unwrap();
                ed25519_dalek::SigningKey::from_bytes(&sk)
            },
            #[cfg(feature = "secp256k1")]
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

#[cfg(feature = "ed25519")]
const _: () = {
    use crate::ed25519::Ed25519;

    impl DeriveSigner<Ed25519> for InMemorySigner {
        fn public_key(&self) -> <Ed25519 as DerivableCurve>::PublicKey {
            self.ed25519_root_sk.public_key()
        }

        fn sign(
            &self,
            tweak: &<Ed25519 as crate::DerivableCurve>::Tweak,
            msg: &[u8],
        ) -> <Ed25519 as crate::DerivableCurve>::Signature {
            self.ed25519_root_sk.sign(tweak, msg)
        }
    }
};

#[cfg(feature = "secp256k1")]
const _: () = {
    use crate::secp256k1::Secp256k1;

    impl DeriveSigner<Secp256k1> for InMemorySigner {
        fn public_key(&self) -> <Secp256k1 as DerivableCurve>::PublicKey {
            self.secp256k1_root_sk.public_key()
        }

        fn sign(
            &self,
            tweak: &<Secp256k1 as crate::DerivableCurve>::Tweak,
            msg: &[u8],
        ) -> <Secp256k1 as crate::DerivableCurve>::Signature {
            self.secp256k1_root_sk.sign(tweak, msg)
        }
    }
};
