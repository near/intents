#[cfg(feature = "signing")]
pub use k256::SecretKey;
pub use k256::{
    NonZeroScalar, PublicKey,
    ecdsa::{RecoveryId, Signature},
};
use k256::{
    ProjectivePoint, U256,
    elliptic_curve::ops::{MulByGenerator, Reduce},
};

use crate::{DerivableCurve, DerivablePublicKey};

pub struct Secp256k1;

impl DerivableCurve for Secp256k1 {
    type Tweak = NonZeroScalar;
    type Signature = (Signature, RecoveryId);

    fn tweak(hash: [u8; 32]) -> Self::Tweak {
        // TODO: are we sure that we need **non-zero** scalar?
        <NonZeroScalar as Reduce<U256>>::reduce_bytes(&hash.into())
    }
}

impl DerivablePublicKey<Secp256k1> for PublicKey {
    fn derive(&self, tweak: <Secp256k1 as DerivableCurve>::Tweak) -> Self {
        // pk' <- pk + G * tweak
        let derived_point = self.to_projective() + ProjectivePoint::mul_by_generator(&tweak);

        // With a random `tweak`, `derived_point == 0` iff `tweak == -root_sk`,
        // which happens with probability ≈ 2^-256 — treat as unreachable.
        // `PublicKey::from_affine` rejects the identity point for us.
        Self::from_affine(derived_point.to_affine())
            .expect("derived public key is the point at infinity")
    }
}

#[cfg(feature = "signing")]
const _: () = {
    use k256::ecdsa::SigningKey;

    use crate::DeriveSigner;

    impl DeriveSigner<Secp256k1> for SecretKey {
        type PublicKey = PublicKey;

        fn public_key(&self) -> Self::PublicKey {
            self.public_key()
        }

        fn sign(
            &self,
            tweak: <Secp256k1 as DerivableCurve>::Tweak,
            prehash: &[u8],
        ) -> <Secp256k1 as DerivableCurve>::Signature {
            let derived_scalar = NonZeroScalar::new(*self.to_nonzero_scalar() + *tweak)
                .expect("derived secret key is zero");

            let sk = SigningKey::from(derived_scalar);

            sk.sign_prehash_recoverable(prehash)
                // TODO: require type-safe 32-byte prehash
                .unwrap()
        }
    }
};

#[cfg(all(test, feature = "signing"))]
mod tests {
    use k256::{
        ecdsa::{VerifyingKey, signature::hazmat::PrehashVerifier},
        elliptic_curve::rand_core::OsRng,
    };

    use crate::tests::test_roundtrip;

    use super::*;

    #[test]
    fn roundtrip() {
        test_roundtrip(
            SecretKey::random(&mut OsRng),
            |pk, prehash, (signature, recovery_id)| {
                let verifying_key = VerifyingKey::from(pk);

                verifying_key
                    .verify_prehash(prehash, &signature)
                    .expect("invalid signature");

                let recovered_key =
                    VerifyingKey::recover_from_prehash(prehash, &signature, recovery_id)
                        .expect("failed to recover verifying key");

                assert_eq!(
                    recovered_key, verifying_key,
                    "invalid recovered verifying key"
                );
            },
        );
    }
}
