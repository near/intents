use defuse_kdf::{
    Additive, Curve, Derive, DeriveExt, DeriveSigner, Ed25519, ReduceScalar, digest::Digest,
};
use sha3::{Digest as _, Sha3_256};

use crate::InMemorySigner;

// use domain-separated hashers to avoid algebraic relations between
// derived keys
pub const DOMAIN_SEPARATOR: &[u8] = b"outlayer/ed25519/derive-tweak/v1";

impl<P> DeriveSigner<Ed25519, P> for InMemorySigner
where
    P: AsRef<[u8]>,
{
    type Schema<'a>
        = Derive<Derive<Additive<Ed25519>, ReduceScalar<Ed25519>>, Digest<Sha3_256>>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        self.ed25519_master_sk
            .schema()
            .derive(ReduceScalar::<Ed25519>::new())
            .derive(domain_schema())
    }

    fn derive_sign(&self, path: P, msg: &[u8]) -> <Ed25519 as Curve>::Signature {
        self.ed25519_master_sk
            .by_ref()
            .derive(ReduceScalar::<Ed25519>::new())
            .derive(domain_schema())
            .derive_sign(path, msg)
    }
}

pub fn domain_schema() -> Digest<Sha3_256> {
    thread_local! {
        // per-thread lazily-initialized hasher with pre-processed domain separator
        static HASHER: Sha3_256 = Sha3_256::new_with_prefix(DOMAIN_SEPARATOR);
    }

    Digest::new(HASHER.with(Clone::clone))
}

#[cfg(test)]
mod tests {
    use defuse_kdf::{Schema, assert_signer_roundtrip, ed25519_dalek::SecretKey};
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"",
        hex!("a3c29ef07fbbf76a4067886ec8b6b0f9bd87956fe0fee6d57315ae021f782d8e")
    )]
    #[case(
        b"test",
        hex!("5c91fa52ca0357892ad29e44cb51ff0b8f6253d4511ab2f4662c4d6bb6b5472d")
    )]
    #[case(
        b"5b74d49e83f4aff956284d5f74270e53d7c55dabc4c28f6ef923fbffc5bfdd1d",
        hex!("211b305b845ca1de61450b80e67922135589c1d31c4b6165fc0d060b29faaf78")
    )]
    fn domain_schema_has_not_changed(#[case] path: impl AsRef<[u8]>, #[case] expected: [u8; 32]) {
        let got = domain_schema().derive_path(path);

        assert_eq!(got, expected, "derived hash has changed");
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

    #[rstest]
    #[case(b"", b"", b"")]
    #[case(b"seed", b"path", b"message")]
    fn roundtrip(#[case] seed: &[u8], #[case] path: &[u8], #[case] msg: &[u8]) {
        assert_signer_roundtrip::<Ed25519, _, _>(&InMemorySigner::from_seed(seed), path, msg);
    }
}
