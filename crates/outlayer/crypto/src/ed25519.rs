use curve25519_dalek::{EdwardsPoint, Scalar};
use ed25519_dalek::{
    Signature, SigningKey, VerifyingKey,
    hazmat::{ExpandedSecretKey, raw_sign},
};
use sha2::Sha512;

use crate::{DerivableCurve, DerivablePublicKey, DerivableSigningKey};

// impl DerivableKey for SigningKey {
//     type PublicKey = VerifyingKey;
//     type Signature = Signature;
//     type Tweak = Scalar;

//     fn root_public_key(&self) -> Self::PublicKey {
//         self.verifying_key()
//     }

//     fn tweak(hash: [u8; 32]) -> Self::Tweak {
//         // TODO: use Scalar::from_hash?
//         Scalar::from_bytes_mod_order(hash)
//     }

//     fn derive_public_key(root: Self::PublicKey, tweak: Self::Tweak) -> Self::PublicKey {
//         let derived_point = root.to_edwards() + EdwardsPoint::mul_base(&tweak);
//         VerifyingKey::from(derived_point)
//     }

//     fn sign_derive(&self, tweak: Self::Tweak, msg: &[u8]) -> Self::Signature {
//         let root_sk = ExpandedSecretKey::from(self.as_bytes());

//         let esk = ExpandedSecretKey {
//             scalar: root_sk.scalar + tweak,
//             // TODO: is it ok to reuse root hash_prefix?
//             hash_prefix: root_sk.hash_prefix,
//         };

//         let verifying_key = VerifyingKey::from(&esk);

//         raw_sign::<Sha512>(&esk, msg, &verifying_key)
//     }

//     fn verify(public_key: Self::PublicKey, msg: &[u8], sig: Self::Signature) -> bool {
//         public_key.verify_strict(msg, &sig).is_ok()
//     }
// }

pub struct Ed25519;

impl DerivableCurve for Ed25519 {
    type Tweak = Scalar;
    type Signature = Signature;

    fn make_tweak(tweak: [u8; 32]) -> Self::Tweak {
        Scalar::from_bytes_mod_order(tweak)
    }
}

impl DerivablePublicKey for VerifyingKey {
    type Curve = Ed25519;

    fn derive_from_tweak(&self, tweak: <Self::Curve as DerivableCurve>::Tweak) -> Self {
        let derived_point = self.to_edwards() + EdwardsPoint::mul_base(&tweak);
        VerifyingKey::from(derived_point)
    }
}

impl DerivableSigningKey for SigningKey {
    type Curve = Ed25519;
    type PublicKey = VerifyingKey;

    fn public_key(&self) -> Self::PublicKey {
        self.verifying_key()
    }

    fn sign_derive(
        &self,
        tweak: <Self::Curve as DerivableCurve>::Tweak,
        msg: &[u8],
    ) -> <Self::Curve as DerivableCurve>::Signature {
        let root_sk = ExpandedSecretKey::from(self.as_bytes());

        let esk = ExpandedSecretKey {
            // TODO: clamp_integer()?
            scalar: root_sk.scalar + tweak,
            // TODO: is it ok to reuse root hash_prefix?
            hash_prefix: root_sk.hash_prefix,
        };

        let verifying_key = VerifyingKey::from(&esk);

        raw_sign::<Sha512>(&esk, msg, &verifying_key)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::test_roundtrip;

    use super::*;

    #[test]
    fn roundtrip() {
        test_roundtrip(SigningKey::from_bytes(&[42u8; 32]), |pk, msg, signature| {
            pk.verify_strict(msg, &signature)
                .expect("invalid signature")
        });
    }
}
