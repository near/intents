use defuse_kdf::{crypto::ed25519::Ed25519, ed25519::Scalar};
use near_mpc_crypto_types::Tweak;

use crate::{NearMpcCurve, sealed::Sealed};

impl NearMpcCurve for Ed25519 {
    fn tweak(tweak: Tweak) -> Scalar {
        // See <https://github.com/near/mpc/blob/1f833a13f70addc34eb1cff704f93fec61e7f7eb/crates/contract/src/crypto_shared/kdf.rs#L36>
        Scalar::from_bytes_mod_order(tweak.as_bytes())
    }
}

impl Sealed for Ed25519 {}

#[cfg(test)]
mod tests {
    use defuse_kdf::{
        DerivableCurve, Schema,
        ed25519::{VerifyingKey, ed25519_dalek},
    };
    use hex_literal::hex;
    use near_account_id::AccountIdRef;
    use rstest::rstest;

    use crate::NearMpcDerivation;

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

        let schema = NearMpcDerivation::<Ed25519>::new(predecessor_id);
        let tweak = schema.derive_path(path);
        let derived_pk = Ed25519::derive_public_key(&master_pk, &tweak);

        assert_eq!(
            derived_pk.to_bytes(),
            expected_derived_pk,
            "derived public key mismatch"
        );
    }
}
