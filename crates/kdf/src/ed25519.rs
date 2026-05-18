use curve25519_dalek::{EdwardsPoint, Scalar};
use defuse_kdf_crypto::Ed25519;
use ed25519_dalek::{
    Digest, Sha512, Signature, SigningKey, VerifyingKey,
    hazmat::{ExpandedSecretKey, raw_sign},
};

use crate::{Additive, CurveArithmetic, DeriveSigner, ReduceScalar, Schema};

impl CurveArithmetic for Ed25519 {
    type Scalar = Scalar;

    type Point = EdwardsPoint;

    fn mul_by_generator(scalar: &Self::Scalar) -> Self::Point {
        EdwardsPoint::mul_base(scalar)
    }

    fn pk2point(public_key: &Self::PublicKey) -> Self::Point {
        public_key.to_edwards()
    }

    fn point2pk(point: Self::Point) -> Self::PublicKey {
        point.into()
    }
}

impl Schema<[u8; 32]> for ReduceScalar<Ed25519> {
    type Output = Scalar;

    #[inline]
    fn derive_path(&self, path: [u8; 32]) -> Self::Output {
        Scalar::from_bytes_mod_order(path)
    }
}

impl Schema<[u8; 64]> for ReduceScalar<Ed25519> {
    type Output = Scalar;

    #[inline]
    fn derive_path(&self, path: [u8; 64]) -> Self::Output {
        Scalar::from_bytes_mod_order_wide(&path)
    }
}

impl DeriveSigner<Ed25519, Scalar> for SigningKey {
    type Schema<'a>
        = Additive<Ed25519>
    where
        Self: 'a;

    #[inline]
    fn schema(&self) -> Self::Schema<'_> {
        Additive::new(self.verifying_key())
    }

    fn derive_sign(&self, tweak: Scalar, msg: &[u8]) -> Signature {
        let esk = ExpandedSecretKey::from(self.as_bytes());

        debug_assert_eq!(
            esk.schema().public_key(),
            self.schema().public_key(),
            "master public key mismatch",
        );

        // delegate signing to expanded secret key
        esk.derive_sign(tweak, msg)
    }
}

impl DeriveSigner<Ed25519, Scalar> for ExpandedSecretKey {
    type Schema<'a>
        = Additive<Ed25519>
    where
        Self: 'a;

    #[inline]
    fn schema(&self) -> Self::Schema<'_> {
        Additive::new(VerifyingKey::from(self))
    }

    fn derive_sign(&self, tweak: Scalar, msg: &[u8]) -> Signature {
        let derived_esk = Self {
            // sk' = sk + tweak
            scalar: self.scalar + tweak,

            // In ed25519-dalek implementation `hash_prefix` takes part in
            // deterministic nonce generation. It's very important to not
            // reuse the same nonce for different challenges, as it might
            // lead to leaking the root private key.
            hash_prefix: {
                const DOMAIN_SEPARATOR: &[u8] = b"outlayer/ed25519/derive-hash_prefix/v1";

                thread_local! {
                    // per-thread lazily-initialized hasher with pre-processed domain separator
                    static HASHER: Sha512 = Sha512::new_with_prefix(DOMAIN_SEPARATOR);
                }

                HASHER
                    .with(Clone::clone)
                    .chain_update(self.hash_prefix)
                    .chain_update(tweak.as_bytes())
                    .finalize()[..32]
                    .try_into()
                    .expect("SHA-512 output is 64 bytes")
            },
        };

        let derived_verifying_key = VerifyingKey::from(&derived_esk);

        debug_assert_eq!(
            derived_verifying_key,
            self.derive_public_key(tweak),
            "derived public key mismatch",
        );

        raw_sign::<Sha512>(&derived_esk, msg, &derived_verifying_key)
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{PUBLIC_KEY_LENGTH, SecretKey};
    use hex_literal::hex;
    use rstest::rstest;

    use crate::{DeriveExt, signer::tests::assert_roundtrip};

    use super::*;

    #[rstest]
    fn roundtrip(
        #[values(
            hex!("c9997b51c4eeb50681a52ae87d30daa6cfafc56fddade04ddeb3e1a670f04987"),
        )]
        root_sk: SecretKey,
        #[values(
            hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
        )]
        tweak: [u8; 32],
        #[values(b"", b"test", b"message")] msg: &[u8],
    ) {
        assert_roundtrip(
            &SigningKey::from_bytes(&root_sk).derive(ReduceScalar::<Ed25519>::new()),
            tweak,
            msg,
        );
    }

    #[rstest]
    #[case(
        hex!("c9997b51c4eeb50681a52ae87d30daa6cfafc56fddade04ddeb3e1a670f04987"),
        hex!("108a8530b779de5245e65e92c3590bc8e87034afa8774e8c7365be3732f4b19e"),
        hex!("abb9efe579ee145410090ec74eb15165e9d8ff708cbef75ac99106d5535362ed"),
    )]
    fn derived_pk_has_not_changed(
        #[case] root_sk: SecretKey,
        #[case] tweak: [u8; 32],
        #[case] expected_derived_pk: [u8; PUBLIC_KEY_LENGTH],
    ) {
        let (derived_pk, _signature) = assert_roundtrip(
            &SigningKey::from_bytes(&root_sk).derive(ReduceScalar::<Ed25519>::new()),
            tweak,
            b"message",
        );
        assert_eq!(
            derived_pk.to_bytes(),
            expected_derived_pk,
            "derived public key has changed"
        );
    }
}
