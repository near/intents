use curve25519_dalek::EdwardsPoint;
pub use curve25519_dalek::Scalar;
pub use defuse_outlayer_crypto::ed25519::*;
pub use ed25519_dalek::{self, SigningKey};
use ed25519_dalek::{
    Digest, Sha512,
    hazmat::{ExpandedSecretKey, raw_sign},
};

use crate::{DerivableCurve, DerivationSchema, DeriveSigner};

impl DerivableCurve for Ed25519 {
    type Tweak = Scalar;

    fn derive_public_key(master_pk: &VerifyingKey, tweak: &Scalar) -> VerifyingKey {
        // pk' <- pk + G * tweak
        let derived_point = master_pk.to_edwards() + EdwardsPoint::mul_base(&tweak);

        VerifyingKey::from(derived_point)
    }
}

impl DerivationSchema<Ed25519, Scalar> for SigningKey {
    type Output = Scalar;

    fn derive_path(&self, path: Scalar) -> Self::Output {
        path
    }
}

impl DeriveSigner<Ed25519, Scalar> for SigningKey {
    fn public_key(&self) -> VerifyingKey {
        self.verifying_key()
    }

    fn derive_sign(&self, tweak: Scalar, msg: &[u8]) -> Signature {
        let esk = ExpandedSecretKey::from(self.as_bytes());

        debug_assert_eq!(
            esk.public_key(),
            self.public_key(),
            "master public key mismatch",
        );

        esk.derive_sign(tweak, msg)
    }
}

impl DerivationSchema<Ed25519, Scalar> for ExpandedSecretKey {
    type Output = Scalar;

    fn derive_path(&self, path: Scalar) -> Self::Output {
        path
    }
}

impl DeriveSigner<Ed25519, Scalar> for ExpandedSecretKey {
    fn public_key(&self) -> VerifyingKey {
        self.into()
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

                Sha512::new_with_prefix(DOMAIN_SEPARATOR)
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

// impl AdditiveDerivationScheme<Ed25519, [u8; 32]> for Additive {
//     fn derive_public_key(master_pk: &VerifyingKey, path: &[u8; 32]) -> VerifyingKey {
//         // TODO: domain-separation hash
//         let tweak = Scalar::from_bytes_mod_order(*path);

//         // pk' <- pk + G * tweak
//         let derived_point = master_pk.to_edwards() + EdwardsPoint::mul_base(&tweak);

//         VerifyingKey::from(derived_point)
//     }
// }

// impl PublicKeyDerivationScheme<[u8; 32]> for Ed25519AdditiveDerivation {
//     type Curve = Ed25519;

//     fn derive_public_key(master_pk: &VerifyingKey, path: &[u8; 32]) -> VerifyingKey {
//         // TODO: domain-separation hash
//         let tweak = Self::tweak(*path);

//         // pk' <- pk + G * tweak
//         let derived_point = master_pk.to_edwards() + EdwardsPoint::mul_base(&tweak);

//         VerifyingKey::from(derived_point)
//     }
// }

// impl DerivablePublicKey<Ed25519Derivation> for VerifyingKey {
//     fn derive_from_tweak(&self, tweak: &<Ed25519Derivation as DerivationScheme>::Tweak) -> Self {
//         todo!()
//     }

//     // fn derive(&self, path: &<Ed25519 as DerivableCurve>::Path) -> Self {
//     //     let tweak = Ed25519::tweak(path);

//     //     // pk' <- pk + G * tweak
//     //     let derived_point = self.to_edwards() + EdwardsPoint::mul_base(&tweak);

//     //     Self::from(derived_point)
//     // }
// }

// #[cfg(feature = "signing")]
// const _: () = {
//     use ed25519_dalek::{
//         Sha512,
//         hazmat::{ExpandedSecretKey, raw_sign},
//     };
//     use rand::{RngExt, rand_core::UnwrapErr, rngs::SysRng};

//     use crate::DeriveSigner;

//     impl DeriveSigner<Ed25519, Additive, [u8; 32]> for SigningKey {
//         fn public_key(&self) -> VerifyingKey {
//             self.verifying_key()
//         }

//         fn derive_sign(&self, path: &[u8; 32], msg: &[u8]) -> Signature {
//             let tweak = Scalar::from_bytes_mod_order(*path);

//             let root_esk = ExpandedSecretKey::from(self.as_bytes());

//             let derived_esk = ExpandedSecretKey {
//                 // sk' = sk + tweak
//                 scalar: root_esk.scalar + tweak,

//                 // In ed25519-dalek implementation hash_prefix takes part in
//                 // deterministic nonce generation. It's very important to not
//                 // reuse the same nonce for different challenges, as it might
//                 // lead to leaking the root private key.
//                 //
//                 // Here we generate the hash, and thus, the nonce randomly.
//                 // As a result, signature will be different every time, even
//                 // if the same message is singed with the same tweak.
//                 //
//                 // TODO: derive hash_prefix deterministically from
//                 // `root_sk.hash_prefix` and `tweak`?
//                 hash_prefix: UnwrapErr::<SysRng>::default().random(),
//             };

//             let derived_verifying_key = VerifyingKey::from(&derived_esk);

//             debug_assert_eq!(
//                 derived_verifying_key,
//                 self.derive_public_key(path),
//                 "derived public key mismatch",
//             );

//             raw_sign::<Sha512>(&derived_esk, msg, &derived_verifying_key)
//         }
//     }

//     // TODO: impl for ExpandedSecretKey
// };

#[cfg(test)]
mod tests {
    use ed25519_dalek::{PUBLIC_KEY_LENGTH, SecretKey};
    use hex_literal::hex;
    use rstest::rstest;

    use crate::{SchemaFn, signer::tests::assert_roundtrip};

    use super::*;

    const SCHEMA: SchemaFn<Ed25519, fn([u8; 32]) -> Scalar> =
        SchemaFn::new(Scalar::from_bytes_mod_order);

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
            &SigningKey::from_bytes(&root_sk),
            SCHEMA.derive_path(tweak),
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
            &SigningKey::from_bytes(&root_sk),
            SCHEMA.derive_path(tweak),
            b"message",
        );
        assert_eq!(
            derived_pk.to_bytes(),
            expected_derived_pk,
            "derived public key has changed"
        );
    }
}
