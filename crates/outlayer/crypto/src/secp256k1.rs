#[cfg(feature = "signing")]
pub use k256::ecdsa::SigningKey;
pub use k256::{
    self,
    ecdsa::{RecoveryId, Signature, VerifyingKey},
};
use k256::{
    NonZeroScalar, ProjectivePoint, U256,
    ecdsa::signature::hazmat::PrehashVerifier,
    elliptic_curve::ops::{MulByGenerator, Reduce},
};

use crate::{DerivableCurve, DerivablePublicKey};

pub struct Secp256k1;

impl Secp256k1 {
    fn tweak(path: &[u8; 32]) -> NonZeroScalar {
        <NonZeroScalar as Reduce<U256>>::reduce_bytes(path.into())
    }
}

impl DerivableCurve for Secp256k1 {
    type Path = [u8; 32];
    type PublicKey = VerifyingKey;
    /// Prehash, i.e. output of a cryptographic hash function
    type Message = [u8; 32];
    type Signature = (Signature, RecoveryId);

    fn verify(
        public_key: &VerifyingKey,
        prehash: &[u8; 32],
        (signature, _recovery_id): &Self::Signature,
    ) -> bool {
        public_key.verify_prehash(prehash, signature).is_ok()
    }
}

impl DerivablePublicKey<Secp256k1> for VerifyingKey {
    fn derive(&self, path: &<Secp256k1 as DerivableCurve>::Path) -> Self {
        let tweak = Secp256k1::tweak(path);

        // pk' <- pk + G * tweak
        let derived_point =
            ProjectivePoint::from(self.as_affine()) + ProjectivePoint::mul_by_generator(&tweak);

        // `PublicKey::from_affine` rejects the identity point for us.
        // With a random `tweak`, `derived_point == 0` iff `tweak == -root_sk`,
        // which happens with probability ≈ 2^-256 — treat as unreachable.
        Self::from_affine(derived_point.to_affine())
            .expect("derived public key is the point at infinity")
    }
}

#[cfg(feature = "signing")]
const _: () = {
    use crate::DeriveSigner;

    impl DeriveSigner<Secp256k1> for SigningKey {
        fn public_key(&self) -> VerifyingKey {
            self.verifying_key().clone()
        }

        fn derive_sign(
            &self,
            path: &<Secp256k1 as DerivableCurve>::Path,
            prehash: &[u8; 32],
        ) -> (Signature, RecoveryId) {
            let tweak = Secp256k1::tweak(path);

            let derived_scalar = NonZeroScalar::new(
                // sk' = sk + tweak
                **self.as_nonzero_scalar() + *tweak,
            )
            .expect("derived secret key is zero");

            let derived_sk = SigningKey::from(derived_scalar);

            debug_assert_eq!(
                derived_sk.verifying_key(),
                &self.derive_public_key(path),
                "derived public key mismatch",
            );

            derived_sk
                .sign_prehash_recoverable(prehash)
                .expect("invalid derived signing key")
        }
    }
};

#[cfg(all(test, feature = "signing"))]
mod tests {
    use hex_literal::hex;
    use k256::{EncodedPoint, ecdsa::VerifyingKey};
    use rstest::rstest;

    use crate::tests::{assert_roundtrip, assert_roundtrip_expected};

    use super::*;

    #[rstest]
    fn roundtrip(
        #[values(
            hex!("bd635d1f79748034dcb9654b5915b1ca94dfd66f6b78c2067f78110a0106af10"),
        )]
        root_sk: [u8; 32],
        #[values(
            hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
        )]
        path: [u8; 32],
        #[values(
            hex!("0000000000000000000000000000000000000000000000000000000000000000"),
            hex!("00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee42"),
        )]
        prehash: [u8; 32],
    ) {
        let (derived_pk, (signature, recovery_id)) = assert_roundtrip(
            SigningKey::from_bytes(&root_sk.into()).expect("invalid root sk"),
            &path,
            &prehash,
        );

        let recovered_key = VerifyingKey::recover_from_prehash(&prehash, &signature, recovery_id)
            .expect("failed to recover verifying key");

        assert_eq!(
            recovered_key,
            derived_pk.into(),
            "invalid recovered verifying key"
        );
    }

    #[rstest]
    #[case(
        hex!("bd635d1f79748034dcb9654b5915b1ca94dfd66f6b78c2067f78110a0106af10"),
        hex!("108a8530b779de5245e65e92c3590bc8e87034afa8774e8c7365be3732f4b19e"),
        hex!("ff0a1347d1aa363e71c1c33c06e10050d3278b0f308b190bdf22bcfce9821344f596012c92bc2adba6f3fa4f98874d70bb2eb1a1bc0441674c14f77ae4c8d214"),
    )]
    fn derived_pk_has_not_changed(
        #[case] root_sk: [u8; 32],
        #[case] path: [u8; 32],
        #[case] expected_derived_pk: [u8; 64],
    ) {
        assert_roundtrip_expected(
            SigningKey::from_bytes(&root_sk.into()).expect("invalid root sk"),
            &path,
            &hex!("00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee42"),
            &VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
                &expected_derived_pk.into(),
            ))
            .expect("invalid expected derived pk"),
        );
    }
}
