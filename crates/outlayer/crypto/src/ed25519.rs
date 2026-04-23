use curve25519_dalek::{EdwardsPoint, Scalar};
use ed25519_dalek::{
    Signature, SigningKey, VerifyingKey,
    hazmat::{ExpandedSecretKey, raw_sign},
};
use sha2::Sha512;

use crate::DerivableKey;

impl DerivableKey for SigningKey {
    type PublicKey = VerifyingKey;
    type Signature = Signature;
    type Tweak = Scalar;

    fn root_public_key(&self) -> Self::PublicKey {
        self.verifying_key()
    }

    fn tweak(hash: [u8; 32]) -> Self::Tweak {
        // TODO: use Scalar::from_hash?
        Scalar::from_bytes_mod_order(hash)
    }

    fn derive_public_key(root: Self::PublicKey, tweak: Self::Tweak) -> Self::PublicKey {
        let derived_point = root.to_edwards() + EdwardsPoint::mul_base(&tweak);
        VerifyingKey::from(derived_point)
    }

    fn sign(&self, tweak: Self::Tweak, msg: &[u8]) -> Self::Signature {
        let root_sk = ExpandedSecretKey::from(self.as_bytes());

        let esk = ExpandedSecretKey {
            scalar: root_sk.scalar + tweak,
            // TODO: is it ok to reuse root hash_prefix?
            hash_prefix: root_sk.hash_prefix,
        };

        let verifying_key = VerifyingKey::from(&esk);

        raw_sign::<Sha512>(&esk, msg, &verifying_key)
    }

    fn verify(public_key: Self::PublicKey, msg: &[u8], sig: Self::Signature) -> bool {
        public_key.verify_strict(msg, &sig).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::test_roundtrip;

    use super::*;

    #[test]
    fn roundtrip() {
        test_roundtrip(SigningKey::from_bytes(&[67u8; 32]));
    }
}
