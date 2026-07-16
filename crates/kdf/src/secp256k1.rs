use defuse_crypto::{RecoverableCurve, secp256k1::Secp256k1};
use k256::{
    FieldBytes, NonZeroScalar, ProjectivePoint, WideBytes,
    ecdsa::{self, RecoveryId, Signature, SigningKey, VerifyingKey},
    elliptic_curve::ops::Reduce,
};

use crate::{
    Additive, CurveArithmetic, DeriveSigner, RecoverableDeriveSigner, ReduceScalar, Schema,
};

impl CurveArithmetic for Secp256k1 {
    type Scalar = NonZeroScalar;

    type Point = ProjectivePoint;

    #[inline]
    fn mul_by_generator(scalar: &Self::Scalar) -> Self::Point {
        ProjectivePoint::mul_by_generator(scalar)
    }

    #[inline]
    fn pk2point(public_key: &Self::PublicKey) -> Self::Point {
        public_key.as_affine().into()
    }

    #[inline]
    fn point2pk(point: Self::Point) -> Self::PublicKey {
        VerifyingKey::from_affine(point.to_affine())
            .expect("derived public key is the point at infinity")
    }
}

impl DeriveSigner<Secp256k1, NonZeroScalar> for SigningKey {
    type Error = ecdsa::Error;

    type Schema<'a>
        = Additive<Secp256k1>
    where
        Self: 'a;

    #[inline]
    fn schema(&self) -> Self::Schema<'_> {
        Additive::new(*self.verifying_key())
    }

    /// Sign given **32-bytes prehash** with _internally_ derived secret key
    async fn derive_sign(
        &self,
        tweak: NonZeroScalar,
        prehash: &[u8],
    ) -> Result<Signature, Self::Error> {
        self.derive_sign_recoverable(tweak, prehash)
            .await
            .map(|s| s.0)
    }
}

impl RecoverableDeriveSigner<Secp256k1, NonZeroScalar> for SigningKey {
    /// Sign given **32-bytes prehash** with _internally_ derived secret key
    /// and return signature along with recovery id.
    async fn derive_sign_recoverable(
        &self,
        tweak: NonZeroScalar,
        prehash: &[u8],
    ) -> Result<(Signature, RecoveryId), Self::Error> {
        let prehash: &[u8; 32] = prehash
            .try_into()
            .map_err(|_| ecdsa::Error::from_source("prehash must be 32-bytes long"))?;

        let derived_scalar = NonZeroScalar::new(
            // sk' = sk + tweak
            **self.as_nonzero_scalar() + *tweak,
        )
        .into_option()
        // derived secret key is zero
        .ok_or_else(ecdsa::Error::new)?;

        let derived_sk = Self::from(derived_scalar);

        debug_assert_eq!(
            derived_sk.verifying_key(),
            &self.derive_public_key(tweak),
            "derived public key mismatch",
        );

        let (sig, recovery_id) = derived_sk.sign_prehash_recoverable(prehash);

        debug_assert_eq!(
            Secp256k1::recover(prehash, &sig, recovery_id),
            Some(self.derive_public_key(tweak)),
            "invalid recovered public key",
        );

        Ok((sig, recovery_id))
    }
}

impl Schema<[u8; 32]> for ReduceScalar<Secp256k1> {
    type Output = NonZeroScalar;

    #[inline]
    fn derive_path(&self, path: [u8; 32]) -> Self::Output {
        Reduce::<FieldBytes>::reduce(&path.into())
    }
}

impl Schema<[u8; 64]> for ReduceScalar<Secp256k1> {
    type Output = NonZeroScalar;

    #[inline]
    fn derive_path(&self, path: [u8; 64]) -> Self::Output {
        Reduce::<WideBytes>::reduce(&path.into())
    }
}

#[cfg(test)]
mod tests {
    use defuse_crypto::secp256k1::Secp256k1UncompressedPublicKey;
    use hex_literal::hex;
    use rstest::rstest;

    use crate::{DeriveExt, signer::assert_signer_roundtrip};

    use super::*;

    #[rstest]
    #[tokio::test]
    async fn roundtrip(
        #[values(
            hex!("bd635d1f79748034dcb9654b5915b1ca94dfd66f6b78c2067f78110a0106af10"),
        )]
        root_sk: [u8; 32],
        #[values(
            hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
        )]
        tweak: [u8; 32],
        #[values(
            hex!("0000000000000000000000000000000000000000000000000000000000000000"),
            hex!("00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee42"),
        )]
        prehash: [u8; 32],
    ) {
        assert_signer_roundtrip(
            &SigningKey::from_bytes(&root_sk.into())
                .expect("invalid root sk")
                .derive(ReduceScalar::<Secp256k1>::new()),
            tweak,
            &prehash,
        )
        .await;
    }

    #[rstest]
    #[case(
        hex!("bd635d1f79748034dcb9654b5915b1ca94dfd66f6b78c2067f78110a0106af10"),
        hex!("108a8530b779de5245e65e92c3590bc8e87034afa8774e8c7365be3732f4b19e"),
        hex!("ff0a1347d1aa363e71c1c33c06e10050d3278b0f308b190bdf22bcfce9821344f596012c92bc2adba6f3fa4f98874d70bb2eb1a1bc0441674c14f77ae4c8d214"),
    )]
    #[tokio::test]
    async fn derived_pk_has_not_changed(
        #[case] root_sk: [u8; 32],
        #[case] tweak: [u8; 32],
        #[case] expected_derived_pk: impl Into<Secp256k1UncompressedPublicKey>,
    ) {
        let (derived_pk, _signature) = assert_signer_roundtrip(
            &SigningKey::from_bytes(&root_sk.into())
                .expect("invalid root sk")
                .derive(ReduceScalar::<Secp256k1>::new()),
            tweak,
            &hex!("00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee42"),
        )
        .await;
        assert_eq!(
            // compress and skip tag byte
            Secp256k1UncompressedPublicKey::from(derived_pk),
            expected_derived_pk.into(),
            "derived public key has changed"
        );
    }
}
