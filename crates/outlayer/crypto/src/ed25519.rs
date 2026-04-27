use curve25519_dalek::{EdwardsPoint, Scalar};
#[cfg(feature = "signing")]
pub use ed25519_dalek::SigningKey;
pub use ed25519_dalek::{self, Signature, VerifyingKey};

use crate::{DerivableCurve, DerivablePublicKey};

pub struct Ed25519;

impl Ed25519 {
    fn tweak(path: &[u8; 32]) -> Scalar {
        // TODO: are we sure there is no need to clamp?
        Scalar::from_bytes_mod_order(*path)
    }
}

impl DerivableCurve for Ed25519 {
    type Path = [u8; 32];
    type PublicKey = VerifyingKey;
    type Message = [u8];
    type Signature = Signature;

    fn verify(public_key: &VerifyingKey, msg: &[u8], signature: &Signature) -> bool {
        public_key.verify_strict(msg, signature).is_ok()
    }
}

impl DerivablePublicKey<Ed25519> for VerifyingKey {
    fn derive(&self, path: &<Ed25519 as DerivableCurve>::Path) -> Self {
        let tweak = Ed25519::tweak(path);

        // pk' <- pk + G * tweak
        let derived_point = self.to_edwards() + EdwardsPoint::mul_base(&tweak);

        Self::from(derived_point)
    }
}

#[cfg(feature = "signing")]
const _: () = {
    use ed25519_dalek::{
        Sha512,
        hazmat::{ExpandedSecretKey, raw_sign},
    };
    use rand::{RngExt, rand_core::UnwrapErr, rngs::SysRng};

    use crate::DeriveSigner;

    impl DeriveSigner<Ed25519> for SigningKey {
        fn public_key(&self) -> VerifyingKey {
            self.verifying_key()
        }

        fn derive_sign(&self, path: &<Ed25519 as DerivableCurve>::Path, msg: &[u8]) -> Signature {
            let tweak = Ed25519::tweak(path);

            let root_esk = ExpandedSecretKey::from(self.as_bytes());

            let derived_esk = ExpandedSecretKey {
                // sk' = sk + tweak
                scalar: root_esk.scalar + tweak,

                // In ed25519-dalek implementation hash_prefix takes part in
                // deterministic nonce generation. It's very important to not
                // reuse the same nonce for different challenges, as it might
                // lead to leaking the root private key.
                //
                // Here we generate the hash, and thus, the nonce randomly.
                // As a result, signature will be different every time, even
                // if the same message is singed with the same tweak.
                //
                // TODO: derive hash_prefix deterministically from
                // `root_sk.hash_prefix` and `tweak`?
                hash_prefix: UnwrapErr::<SysRng>::default().random(),
            };

            let derived_verifying_key = VerifyingKey::from(&derived_esk);

            debug_assert_eq!(
                derived_verifying_key,
                self.derive_public_key(path),
                "derived public key mismatch",
            );

            raw_sign::<Sha512>(&derived_esk, msg, &derived_verifying_key)
        }
    }
};

#[cfg(all(test, feature = "signing"))]
mod tests {
    use ed25519_dalek::{PUBLIC_KEY_LENGTH, SecretKey, SigningKey, VerifyingKey};
    use hex_literal::hex;
    use rstest::rstest;

    use crate::tests::{assert_roundtrip, assert_roundtrip_expected};

    #[rstest]
    fn roundtrip(
        #[values(
            hex!("c9997b51c4eeb50681a52ae87d30daa6cfafc56fddade04ddeb3e1a670f04987"),
        )]
        root_sk: SecretKey,
        #[values(
            hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"),
        )]
        path: [u8; 32],
        #[values(b"", b"test", b"message")] msg: &[u8],
    ) {
        assert_roundtrip(SigningKey::from_bytes(&root_sk), &path, msg);
    }

    #[rstest]
    #[case(
        hex!("c9997b51c4eeb50681a52ae87d30daa6cfafc56fddade04ddeb3e1a670f04987"),
        hex!("108a8530b779de5245e65e92c3590bc8e87034afa8774e8c7365be3732f4b19e"),
        hex!("abb9efe579ee145410090ec74eb15165e9d8ff708cbef75ac99106d5535362ed"),
    )]
    fn derived_pk_has_not_changed(
        #[case] root_sk: SecretKey,
        #[case] path: [u8; 32],
        #[case] expected_derived_pk: [u8; PUBLIC_KEY_LENGTH],
    ) {
        assert_roundtrip_expected(
            SigningKey::from_bytes(&root_sk),
            &path,
            b"message",
            &VerifyingKey::from_bytes(&expected_derived_pk).expect("invalid expected derived pk"),
        );
    }
}
