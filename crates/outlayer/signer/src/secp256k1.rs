use defuse_kdf::{
    Additive, Curve, Derive, DeriveExt, DeriveSigner, ReduceScalar, Secp256k1, digest::Digest,
};
use sha3::{Digest as _, Sha3_256};

use crate::InMemorySigner;

// use domain-separated hashers to avoid algebraic relations between
// derived keys
pub const DOMAIN_SEPARATOR: &[u8] = b"outlayer/secp256k1/derive-tweak/v1";

impl<P> DeriveSigner<Secp256k1, P> for InMemorySigner
where
    P: AsRef<[u8]>,
{
    type Schema<'a>
        = Derive<Derive<Additive<Secp256k1>, ReduceScalar<Secp256k1>>, Digest<Sha3_256>>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        self.secp256k1_master_sk
            .schema()
            .derive(ReduceScalar::<Secp256k1>::new())
            .derive(domain_schema())
    }

    fn derive_sign(&self, path: P, msg: &[u8; 32]) -> <Secp256k1 as Curve>::Signature {
        self.secp256k1_master_sk
            .by_ref()
            .derive(ReduceScalar::<Secp256k1>::new())
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
    use defuse_kdf::{Schema, assert_signer_roundtrip};
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"",
        hex!("e1795f919b6520cd07517f9baacb34f804e785363643ae7991732e2d2e93f99d")
    )]
    #[case(
        b"test",
        hex!("84be8d88cfae2c03949f9dea5c96d3cc6b6017274329cd7dc65d396660fce274")
    )]
    #[case(
        b"5b74d49e83f4aff956284d5f74270e53d7c55dabc4c28f6ef923fbffc5bfdd1d",
        hex!("f255d58f3ff729c23ed44f95f1e242e02600f9ffef479db79612659e7f9551e5")
    )]
    fn domain_schema_has_not_changed(#[case] path: impl AsRef<[u8]>, #[case] expected: [u8; 32]) {
        let got = domain_schema().derive_path(path);

        assert_eq!(got, expected, "derived hash has changed");
    }

    #[rstest]
    #[case(
        b"",
        hex!("954787a5fb30c67cb33717beecc4c0378e76f1142b6d5b7f9d168baa3a02c166"),
    )]
    #[case(
        b"test",
        hex!("fe897ab00ec1763cde63c7d96ecb841402c8f62dbb5d2ff5f8fea8e500a92c2d"),
    )]
    #[case(
        &hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
        hex!("64803a99628fd19eb82cc7cc8a41d9f1745eb7f7f1e2887058d3b922db93f74e"),
    )]
    fn seed_derivation_has_not_changed(
        #[case] seed: &[u8],
        #[case] expected_secp256k1_sk: [u8; 32],
    ) {
        let derived = InMemorySigner::from_seed(seed);
        assert_eq!(
            *derived.secp256k1_master_sk.to_bytes(),
            expected_secp256k1_sk,
            "secp256k1 derivation has changed"
        );
    }

    #[rstest]
    #[case(
        b"",
        b"",
        hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
    )]
    #[case(
        b"seed",
        b"path",
        hex!("00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee42"),
    )]
    fn roundtrip(#[case] seed: &[u8], #[case] path: &[u8], #[case] msg: [u8; 32]) {
        assert_signer_roundtrip::<Secp256k1, _, _>(&InMemorySigner::from_seed(seed), path, &msg);
    }
}
