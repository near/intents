use defuse_outlayer_crypto::{
    ed25519::{self, Ed25519, ed25519_dalek},
    secp256k1::{self, Secp256k1},
};
use defuse_outlayer_kdf::{Curve, DerivationScheme, DeriveSigner};
use hkdf::Hkdf;
use sha3::Sha3_512;

#[cfg_attr(feature = "zeroize", derive(::zeroize::ZeroizeOnDrop))]
#[derive(Clone, PartialEq, Eq)]
pub struct InMemorySigner {
    ed25519_master_sk: ed25519::SigningKey,
    secp256k1_master_sk: secp256k1::SigningKey,
}

impl InMemorySigner {
    const HKDF_SEED_SALT: &'static [u8] = b"outlayer v0.1.0 signer seed:";

    const HKDF_INFO_ED25519_ROOT_SK: &'static [u8] = b"ed25519/root_sk";
    const HKDF_INFO_SECP256K1_ROOT_SK: &'static [u8] = b"secp256k1/root_sk";

    /// Construct from a seed, i.e. input key material with _not necessarily_
    /// uniformly distributed entropy.
    ///
    /// NOTE: `seed` is passed by reference, and the referenced value is
    /// recommended to be zeroized afterwards.
    pub fn from_seed(seed: &[u8]) -> Self {
        let hk = Hkdf::<Sha3_512>::new(Some(Self::HKDF_SEED_SALT), seed);

        Self {
            ed25519_master_sk: {
                let mut sk = [0u8; ed25519_dalek::SECRET_KEY_LENGTH];
                hk.expand(Self::HKDF_INFO_ED25519_ROOT_SK, &mut sk)
                    .expect("HKDF: ed25519");
                crate::ed25519::SigningKey::from_bytes(&sk)
            },
            secp256k1_master_sk: {
                let mut sk = [0u8; 32];
                hk.expand(Self::HKDF_INFO_SECP256K1_ROOT_SK, &mut sk)
                    .expect("HKDF: secp256k1");
                crate::secp256k1::SigningKey::from_bytes(&sk.into())
                    .expect("secp256k1: zero scalar")
            },
        }
    }
}

impl<S, P> DeriveSigner<Ed25519, S, P> for InMemorySigner
where
    S: DerivationScheme<Ed25519, P> + ?Sized,
{
    fn public_key(&self) -> <Ed25519 as Curve>::PublicKey {
        DeriveSigner::<Ed25519, S, P>::public_key(&self.ed25519_master_sk)
    }

    fn derive_sign(&self, path: P, msg: &[u8]) -> <Ed25519 as Curve>::Signature {
        DeriveSigner::<Ed25519, S, P>::derive_sign(&self.ed25519_master_sk, path, msg)
    }
}

impl<S, P> DeriveSigner<Secp256k1, S, P> for InMemorySigner
where
    S: DerivationScheme<Secp256k1, P> + ?Sized,
{
    fn public_key(&self) -> <Secp256k1 as Curve>::PublicKey {
        DeriveSigner::<Secp256k1, S, P>::public_key(&self.secp256k1_master_sk)
    }

    fn derive_sign(&self, path: P, msg: &[u8; 32]) -> <Secp256k1 as Curve>::Signature {
        DeriveSigner::<Secp256k1, S, P>::derive_sign(&self.secp256k1_master_sk, path, msg)
    }
}

// const _: () = {
//     use crate::ed25519::{Ed25519, VerifyingKey};

//     impl DerivationScheme<Ed25519, [u8; 32]> for InMemorySigner {
//         fn derive_public_key(&self, path: &[u8; 32]) -> VerifyingKey {
//             self.ed25519_master_sk.derive_public_key(path)
//         }
//     }

//     impl DeriveSigner<Ed25519, [u8; 32]> for InMemorySigner {
//         fn derive_sign(
//             &self,
//             path: &[u8; 32],
//             msg: &<Ed25519 as Curve>::Message,
//         ) -> <Ed25519 as Curve>::Signature {
//             self.ed25519_master_sk.derive_sign(path, msg)
//         }
//     }
// };

// const _: () = {
//     use crate::secp256k1::{Secp256k1, VerifyingKey};

//     impl DerivationScheme<Secp256k1, [u8; 32]> for InMemorySigner {
//         fn derive_public_key(&self, path: &[u8; 32]) -> VerifyingKey {
//             self.secp256k1_master_sk.derive_public_key(path)
//         }
//     }

//     impl DeriveSigner<Secp256k1, [u8; 32]> for InMemorySigner {
//         fn derive_sign(
//             &self,
//             path: &[u8; 32],
//             msg: &<Secp256k1 as Curve>::Message,
//         ) -> <Secp256k1 as Curve>::Signature {
//             self.secp256k1_master_sk.derive_sign(path, msg)
//         }
//     }
// };

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"",
        InMemorySigner {
            ed25519_master_sk: crate::ed25519::SigningKey::from_bytes(
                &hex!("87841790a661e9258a220a23598b1a15f54a8aaac9db0d160918153f8004c008"),
            ),
            secp256k1_master_sk: crate::secp256k1::SigningKey::from_bytes(
                &hex!("954787a5fb30c67cb33717beecc4c0378e76f1142b6d5b7f9d168baa3a02c166").into(),
            ).unwrap(),
        }
    )]
    #[case(
        b"test",
        InMemorySigner {
            ed25519_master_sk: crate::ed25519::SigningKey::from_bytes(
                &hex!("7fef7fed7d4ef1f7b566c0d8eb25c979295279bf733c3e41aeb731a572f63a28"),
            ),
            secp256k1_master_sk: crate::secp256k1::SigningKey::from_bytes(
                &hex!("fe897ab00ec1763cde63c7d96ecb841402c8f62dbb5d2ff5f8fea8e500a92c2d").into(),
            ).unwrap(),
        }
    )]
    #[case(
        &hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
        InMemorySigner {
            ed25519_master_sk: crate::ed25519::SigningKey::from_bytes(
                &hex!("c846458ca3667e46a7dc2814713b99ef4b441523bbe2d286808025b10dab07a0"),
            ),
            secp256k1_master_sk: crate::secp256k1::SigningKey::from_bytes(
                &hex!("64803a99628fd19eb82cc7cc8a41d9f1745eb7f7f1e2887058d3b922db93f74e").into(),
            ).unwrap(),
        }
    )]
    fn seed_derivation_has_not_changed(#[case] seed: &[u8], #[case] expected: InMemorySigner) {
        let derived = InMemorySigner::from_seed(seed);
        assert!(derived == expected, "seed derivation has changed");
    }
}
