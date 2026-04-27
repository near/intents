#![cfg(any(feature = "ed25519", feature = "secp256k1"))]

use hkdf::Hkdf;
use sha3::Sha3_512;
use zeroize::ZeroizeOnDrop;

use crate::{DerivableCurve, DeriveSigner};

#[derive(Clone, ZeroizeOnDrop, PartialEq, Eq)]
pub struct InMemorySigner {
    #[cfg(feature = "ed25519")]
    ed25519_root_sk: crate::ed25519::SigningKey,
    #[cfg(feature = "secp256k1")]
    secp256k1_root_sk: crate::secp256k1::SecretKey,
}

impl InMemorySigner {
    const HKDF_SEED_SALT: &'static [u8] = b"outlayer v0.1.0 signer seed:";

    #[cfg(feature = "ed25519")]
    const HKDF_INFO_ED25519_ROOT_SK: &'static [u8] = b"ed25519/root_sk";
    #[cfg(feature = "secp256k1")]
    const HKDF_INFO_SECP256K1_ROOT_SK: &'static [u8] = b"secp256k1/root_sk";

    pub fn from_seed(seed: &[u8]) -> Self {
        let hk = Hkdf::<Sha3_512>::new(Some(Self::HKDF_SEED_SALT), seed);

        Self {
            #[cfg(feature = "ed25519")]
            ed25519_root_sk: {
                let mut sk = [0u8; ed25519_dalek::SECRET_KEY_LENGTH];
                hk.expand(Self::HKDF_INFO_ED25519_ROOT_SK, &mut sk)
                    .expect("HKDF: ed25519");
                ed25519_dalek::SigningKey::from_bytes(&sk)
            },
            #[cfg(feature = "secp256k1")]
            secp256k1_root_sk: {
                let mut sk = [0u8; 32];
                hk.expand(Self::HKDF_INFO_SECP256K1_ROOT_SK, &mut sk)
                    .expect("HKDF: expand");
                k256::SecretKey::from_bytes(&sk.into()).expect("secp256k1: zero scalar")
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

        fn derive_sign(
            &self,
            tweak: &<Ed25519 as DerivableCurve>::Path,
            msg: &<Ed25519 as DerivableCurve>::Message,
        ) -> <Ed25519 as DerivableCurve>::Signature {
            self.ed25519_root_sk.derive_sign(tweak, msg)
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

        fn derive_sign(
            &self,
            tweak: &<Secp256k1 as DerivableCurve>::Path,
            msg: &<Secp256k1 as DerivableCurve>::Message,
        ) -> <Secp256k1 as DerivableCurve>::Signature {
            self.secp256k1_root_sk.derive_sign(tweak, msg)
        }
    }
};

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"",
        InMemorySigner {
            #[cfg(feature = "ed25519")]
            ed25519_root_sk: ed25519_dalek::SigningKey::from_bytes(
                &hex!("87841790a661e9258a220a23598b1a15f54a8aaac9db0d160918153f8004c008"),
            ),
            #[cfg(feature = "secp256k1")]
            secp256k1_root_sk: k256::SecretKey::from_bytes(
                &hex!("954787a5fb30c67cb33717beecc4c0378e76f1142b6d5b7f9d168baa3a02c166").into(),
            ).unwrap(),
        }
    )]
    #[case(
        b"test",
        InMemorySigner {
            #[cfg(feature = "ed25519")]
            ed25519_root_sk: ed25519_dalek::SigningKey::from_bytes(
                &hex!("7fef7fed7d4ef1f7b566c0d8eb25c979295279bf733c3e41aeb731a572f63a28"),
            ),
            #[cfg(feature = "secp256k1")]
            secp256k1_root_sk: k256::SecretKey::from_bytes(
                &hex!("fe897ab00ec1763cde63c7d96ecb841402c8f62dbb5d2ff5f8fea8e500a92c2d").into(),
            ).unwrap(),
        }
    )]
    #[case(
        &hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
        InMemorySigner {
            #[cfg(feature = "ed25519")]
            ed25519_root_sk: ed25519_dalek::SigningKey::from_bytes(
                &hex!("c846458ca3667e46a7dc2814713b99ef4b441523bbe2d286808025b10dab07a0"),
            ),
            #[cfg(feature = "secp256k1")]
            secp256k1_root_sk: k256::SecretKey::from_bytes(
                &hex!("64803a99628fd19eb82cc7cc8a41d9f1745eb7f7f1e2887058d3b922db93f74e").into(),
            ).unwrap(),
        }
    )]
    fn seed_derivation_has_not_changed(#[case] seed: &[u8], #[case] expected: InMemorySigner) {
        let derived = InMemorySigner::from_seed(seed);
        assert!(derived == expected, "seed derivation has changed");
    }
}
