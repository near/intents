#[cfg(feature = "signing")]
pub use k256::SecretKey;
pub use k256::{
    self, NonZeroScalar, PublicKey,
    ecdsa::{RecoveryId, Signature},
};
use k256::{
    ProjectivePoint, U256,
    ecdsa::{VerifyingKey, signature::hazmat::PrehashVerifier},
    elliptic_curve::ops::{MulByGenerator, Reduce},
};

use crate::DerivableCurve;

pub struct Secp256k1;

impl DerivableCurve for Secp256k1 {
    type Tweak = NonZeroScalar;
    type PublicKey = PublicKey;
    type Signature = (Signature, RecoveryId);

    fn tweak(hash: [u8; 32]) -> Self::Tweak {
        // TODO: are we sure that we need **non-zero** scalar?
        <NonZeroScalar as Reduce<U256>>::reduce_bytes(&hash.into())
    }

    fn derive_public_key(root: &Self::PublicKey, tweak: &Self::Tweak) -> Self::PublicKey {
        // pk' <- pk + G * tweak
        let derived_point = root.to_projective() + ProjectivePoint::mul_by_generator(tweak);

        // With a random `tweak`, `derived_point == 0` iff `tweak == -root_sk`,
        // which happens with probability ≈ 2^-256 — treat as unreachable.
        // `PublicKey::from_affine` rejects the identity point for us.
        PublicKey::from_affine(derived_point.to_affine())
            .expect("derived public key is the point at infinity")
    }

    fn verify(
        public_key: &Self::PublicKey,
        // TODO: type-safe
        prehash: &[u8],
        (signature, _recovery_id): &Self::Signature,
    ) -> bool {
        VerifyingKey::from(public_key)
            .verify_prehash(prehash, signature)
            .is_ok()
    }
}

#[cfg(feature = "signing")]
const _: () = {
    use k256::ecdsa::SigningKey;

    use crate::DeriveSigner;

    impl DeriveSigner<Secp256k1> for SecretKey {
        fn public_key(&self) -> <Secp256k1 as DerivableCurve>::PublicKey {
            self.public_key().into()
        }

        fn sign(
            &self,
            tweak: &<Secp256k1 as DerivableCurve>::Tweak,
            prehash: &[u8],
        ) -> <Secp256k1 as DerivableCurve>::Signature {
            let derived_scalar = NonZeroScalar::new(*self.to_nonzero_scalar() + **tweak)
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
    use k256::{ecdsa::VerifyingKey, elliptic_curve::rand_core::OsRng};

    use crate::{DeriveSigner, tests::test_roundtrip};

    use super::*;

    #[test]
    fn roundtrip() {
        test_roundtrip(SecretKey::random(&mut OsRng));
    }

    #[test]
    fn ecrecover() {
        let root_sk = SecretKey::random(&mut OsRng);

        let tweak = Secp256k1::tweak([42u8; 32]);
        let derived_pk = Secp256k1::derive_public_key(&root_sk.public_key(), &tweak);

        let prehash = [13u8; 32]; // TODO
        let (signature, recovery_id) = root_sk.sign(&tweak, &prehash);

        let recovered_key = VerifyingKey::recover_from_prehash(&prehash, &signature, recovery_id)
            .expect("failed to recover verifying key");

        assert_eq!(
            recovered_key,
            derived_pk.into(),
            "invalid recovered verifying key"
        );
    }
}
