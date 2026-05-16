pub mod ed25519;
pub mod secp256k1;

use std::marker::PhantomData;

pub use defuse_outlayer_kdf as kdf;
use defuse_outlayer_kdf::{DerivableCurve, DerivationSchema};

use hkdf::Hkdf;
use sha3::{Digest, Sha3_256, Sha3_512};

// TODO: docs
#[cfg_attr(feature = "zeroize", derive(::zeroize::ZeroizeOnDrop))]
#[derive(Clone, PartialEq, Eq)]
pub struct InMemorySigner {
    ed25519_master_sk: ed25519::SigningKey,
    secp256k1_master_sk: secp256k1::SigningKey,
}

impl InMemorySigner {
    const HKDF_SEED_SALT: &'static [u8] = b"outlayer v0.1.0 signer seed:";

    // TODO: features
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
                let mut sk = [0u8; ed25519::ed25519_dalek::SECRET_KEY_LENGTH];
                hk.expand(Self::HKDF_INFO_ED25519_ROOT_SK, &mut sk)
                    .expect("HKDF: ed25519");
                ed25519::SigningKey::from_bytes(&sk)
            },
            secp256k1_master_sk: {
                let mut sk = [0u8; 32];
                hk.expand(Self::HKDF_INFO_SECP256K1_ROOT_SK, &mut sk)
                    .expect("HKDF: secp256k1");
                secp256k1::SigningKey::from_bytes(&sk.into()).expect("secp256k1: zero scalar")
            },
        }
    }
}

// TODO: docs
pub struct Schema<C>(PhantomData<C>);

impl<C, P> DerivationSchema<P> for Schema<C>
where
    C: DomainCurve,
    P: AsRef<[u8]>,
{
    type Output = C::Tweak;

    fn derive_path(&self, path: P) -> Self::Output {
        let path: [u8; 32] = Sha3_256::new_with_prefix(C::DOMAIN_SEPARATOR)
            .chain_update(path)
            .finalize()
            .into();

        C::ToTweak::default().derive_path(path)
    }
}

impl<C> Default for Schema<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

// TODO: docs
pub trait DomainCurve: DerivableCurve + sealed::Sealed {
    /// Domain separator to avoid algebraic relations between derived keys
    const DOMAIN_SEPARATOR: &[u8];

    ///
    type ToTweak: DerivationSchema<[u8; 32], Output = Self::Tweak> + Default;
}

mod sealed {
    pub trait Sealed {}
}

#[cfg(test)]
mod tests {

    use defuse_outlayer_kdf::ed25519::ed25519_dalek;
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"",
        hex!("87841790a661e9258a220a23598b1a15f54a8aaac9db0d160918153f8004c008"),
        hex!("954787a5fb30c67cb33717beecc4c0378e76f1142b6d5b7f9d168baa3a02c166"),
    )]
    #[case(
        b"test",
        hex!("7fef7fed7d4ef1f7b566c0d8eb25c979295279bf733c3e41aeb731a572f63a28"),
        hex!("fe897ab00ec1763cde63c7d96ecb841402c8f62dbb5d2ff5f8fea8e500a92c2d"),
    )]
    #[case(
        &hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
        hex!("c846458ca3667e46a7dc2814713b99ef4b441523bbe2d286808025b10dab07a0"),
        hex!("64803a99628fd19eb82cc7cc8a41d9f1745eb7f7f1e2887058d3b922db93f74e"),
    )]
    fn seed_derivation_has_not_changed(
        #[case] seed: &[u8],
        #[case] expected_ed25519_sk: [u8; ed25519_dalek::SECRET_KEY_LENGTH],
        #[case] expected_secp256k1_sk: [u8; 32],
    ) {
        let derived = InMemorySigner::from_seed(seed);
        assert_eq!(
            derived.ed25519_master_sk.as_bytes(),
            &expected_ed25519_sk,
            "ed25519 derivation has changed"
        );
        assert_eq!(
            *derived.secp256k1_master_sk.to_bytes(),
            expected_secp256k1_sk,
            "secp256k1 derivation has changed"
        );
    }
}
