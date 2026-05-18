use defuse_kdf::{
    Additive, Curve, Derive, DeriveExt, DeriveSigner, Ed25519, ReduceScalar, Schema,
    curve25519_dalek::Scalar,
};
use sha3::{Digest, Sha3_256};

use crate::{CurveSchema, InMemorySigner};

impl<P> DeriveSigner<Ed25519, P> for InMemorySigner
where
    P: AsRef<[u8]>,
{
    type Schema<'a>
        = Derive<Additive<Ed25519>, CurveSchema<Ed25519>>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        Additive::new(self.ed25519_master_sk.verifying_key()).derive(CurveSchema::new())
    }

    fn derive_sign(&self, path: P, msg: &[u8]) -> <Ed25519 as Curve>::Signature {
        let tweak = CurveSchema::<Ed25519>::new().derive_path(path);
        self.ed25519_master_sk.derive_sign(tweak, msg)
    }
}

impl<P> Schema<P> for CurveSchema<Ed25519>
where
    P: AsRef<[u8]>,
{
    type Output = Scalar;

    fn derive_path(&self, path: P) -> Self::Output {
        // use domain-separated hashers to avoid algebraic relations between
        // derived keys
        const DOMAIN_SEPARATOR: &[u8] = b"outlayer/ed25519/derive-tweak/v1";

        thread_local! {
            // per-thread lazily-initialized hasher with pre-processed domain separator
            static HASHER: Sha3_256 = Sha3_256::new_with_prefix(DOMAIN_SEPARATOR);
        }

        let hasher = HASHER.with(Clone::clone);

        let path: [u8; 32] = hasher.chain_update(path).finalize().into();

        ReduceScalar::<Ed25519>::default().derive_path(path)
    }
}

#[cfg(test)]
mod tests {
    use defuse_kdf::{Schema, ed25519_dalek::SecretKey};
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"",
        hex!("3b23f008ada264aa8d80cb56d3e7b852bd87956fe0fee6d57315ae021f782d0e")
    )]
    #[case(
        b"test",
        hex!("82e90e99953d32d97d98affe0d5e41e28e6253d4511ab2f4662c4d6bb6b5470d")
    )]
    #[case(
        b"5b74d49e83f4aff956284d5f74270e53d7c55dabc4c28f6ef923fbffc5bfdd1d",
        hex!("a64f77d0cba6207685fb450bd0a409815489c1d31c4b6165fc0d060b29faaf08")
    )]
    fn tweak_derivation_schema_has_not_changed(
        #[case] path: impl AsRef<[u8]>,
        #[case] expected_tweak: [u8; 32],
    ) {
        let got = CurveSchema::<Ed25519>::default().derive_path(path);

        assert_eq!(got.to_bytes(), expected_tweak, "derived tweak has changed");
    }

    #[rstest]
    #[case(
        b"",
        hex!
        ("87841790a661e9258a220a23598b1a15f54a8aaac9db0d160918153f8004c008"),
    )]
    #[case(
        b"test",
        hex!("7fef7fed7d4ef1f7b566c0d8eb25c979295279bf733c3e41aeb731a572f63a28"),
    )]
    #[case(
        &hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
        hex!("c846458ca3667e46a7dc2814713b99ef4b441523bbe2d286808025b10dab07a0"),
    )]
    fn seed_derivation_has_not_changed(
        #[case] seed: &[u8],
        #[case] expected_ed25519_sk: SecretKey,
    ) {
        let derived = InMemorySigner::from_seed(seed);
        assert_eq!(
            derived.ed25519_master_sk.as_bytes(),
            &expected_ed25519_sk,
            "ed25519 derivation has changed"
        );
    }
}
