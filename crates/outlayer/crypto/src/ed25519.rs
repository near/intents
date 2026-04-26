use curve25519_dalek::EdwardsPoint;
pub use curve25519_dalek::Scalar;
#[cfg(feature = "signing")]
pub use ed25519_dalek::SigningKey;
pub use ed25519_dalek::{Signature, VerifyingKey};

use crate::{DerivableCurve, DerivablePublicKey};

pub struct Ed25519;

impl DerivableCurve for Ed25519 {
    type Tweak = Scalar;
    type Signature = Signature;

    fn tweak(hash: [u8; 32]) -> Self::Tweak {
        Scalar::from_bytes_mod_order(hash)
    }
}

impl DerivablePublicKey<Ed25519> for VerifyingKey {
    fn derive(&self, tweak: <Ed25519 as DerivableCurve>::Tweak) -> Self {
        let derived_point = self.to_edwards() + EdwardsPoint::mul_base(&tweak);
        Self::from(derived_point)
    }
}

#[cfg(feature = "signing")]
const _: () = {
    use ed25519_dalek::{Sha512, hazmat::ExpandedSecretKey};

    use crate::DeriveSigner;

    impl DeriveSigner<Ed25519> for SigningKey {
        type PublicKey = VerifyingKey;

        fn public_key(&self) -> Self::PublicKey {
            self.verifying_key()
        }

        fn sign(
            &self,
            tweak: <Ed25519 as DerivableCurve>::Tweak,
            msg: &[u8],
        ) -> <Ed25519 as DerivableCurve>::Signature {
            use ed25519_dalek::hazmat::raw_sign;

            let root_sk = ExpandedSecretKey::from(self.as_bytes());

            let esk = ExpandedSecretKey {
                // TODO: are we sure we don't need to clamp_integer() here?
                // On the other hand, it seems as it would break the verification,
                // since following equation will no longer hold:
                //   pk = sk * G
                scalar: root_sk.scalar + tweak,
                // TODO: is it ok to reuse root hash_prefix? or should we
                // deterministically derive it from (root_sk.hash_prefix, tweak)?
                hash_prefix: root_sk.hash_prefix,
            };

            let verifying_key = VerifyingKey::from(&esk);

            raw_sign::<Sha512>(&esk, msg, &verifying_key)
        }
    }
};

#[cfg(all(test, feature = "signing"))]
mod tests {
    use ed25519_dalek::SigningKey;

    use crate::tests::test_roundtrip;

    #[test]
    fn roundtrip() {
        test_roundtrip(SigningKey::from_bytes(&[42u8; 32]), |pk, msg, signature| {
            pk.verify_strict(msg, &signature)
                .expect("invalid signature");
        });
    }
}
