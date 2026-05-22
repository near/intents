use defuse_kdf::{Ed25519, curve25519_dalek::Scalar};

use super::{NearMpcCurve, sealed::Sealed};

impl NearMpcCurve for Ed25519 {
    fn to_scalar(tweak: [u8; 32]) -> Scalar {
        // See <https://github.com/near/mpc/blob/1f833a13f70addc34eb1cff704f93fec61e7f7eb/crates/contract/src/crypto_shared/kdf.rs#L36>
        Scalar::from_bytes_mod_order(tweak)
    }
}

impl Sealed for Ed25519 {}

#[cfg(test)]
mod tests {
    use defuse_kdf::{
        Additive, DeriveExt, Schema,
        ed25519_dalek::{self, VerifyingKey},
    };
    use hex_literal::hex;
    use near_account_id::AccountIdRef;
    use rstest::rstest;

    use crate::tweak::derive_scalar;

    use super::*;

    type PublicKey = [u8; ed25519_dalek::PUBLIC_KEY_LENGTH];

    const ED25519_MPC_PK: PublicKey =
        hex!("e11a3f20b86cc94e5b7537d866d1c46ae3e41a46a1d08f7aea1696a4a500bfd0");

    #[rstest]
    #[case(
        "0s5256ea13dc110785bddd6694107ef33709369c7f",
        "",
        hex!("248de6264ddaf862233651d89d7282da5ca8233a3db28f6e8b8a26417681323d"),
    )]
    #[case(
        "test",
        "test",
        hex!("5a64da534fc721c4566196ac0ba3a06ac8edecf9e185f980fa67f749d01f7769"),
    )]
    #[case(
        "predecessor",
        "path",
        hex!("2d0feef200659b03b7ef2f3fb7b8659fb3db373497cbefeb8280f0758ca82861"),
    )]
    fn check_derivation(
        #[case] predecessor_id: &'static str,
        #[case] path: &str,
        #[case] expected_derived_pk: PublicKey,
    ) {
        let master_pk = VerifyingKey::from_bytes(&ED25519_MPC_PK).unwrap();
        let predecessor_id = AccountIdRef::new(predecessor_id).unwrap();

        let schema =
            Additive::<Ed25519>::new(master_pk).derive(derive_scalar::<Ed25519>(predecessor_id));

        let derived_pk = schema.derive_path(path);

        assert_eq!(
            derived_pk.to_bytes(),
            expected_derived_pk,
            "derived public key mismatch"
        );
    }
}
