use defuse_kdf::{crypto::secp256k1::Secp256k1, secp256k1::NonZeroScalar};
use near_mpc_crypto_types::Tweak;

use crate::{NearMpcCurve, sealed::Sealed};

impl NearMpcCurve for Secp256k1 {
    fn tweak(tweak: Tweak) -> NonZeroScalar {
        // See <https://github.com/near/mpc/blob/1f833a13f70addc34eb1cff704f93fec61e7f7eb/crates/contract/src/crypto_shared/kdf.rs#L22>.
        NonZeroScalar::from_repr(tweak.as_bytes().into())
            .into_option()
            .expect("tweak is not on curve or zero")
    }
}

impl Sealed for Secp256k1 {}

#[cfg(test)]
mod tests {
    use defuse_kdf::{
        DerivableCurve, Schema,
        secp256k1::{VerifyingKey, k256::EncodedPoint},
    };
    use hex_literal::hex;
    use near_account_id::AccountIdRef;
    use rstest::rstest;

    use crate::NearMpcDerivation;

    use super::*;

    type PublicKey = [u8; 64];

    const SECP256K1_MPC_PK: PublicKey = hex!(
        "903a9a9933ed92bdda3fcf30ac999060a5a0fa51c2b6c74838d3029a5aadefe038f7e4a91714f42bb5a2459a0d294be0cd047b4a999d6fd912702470f843271d"
    );

    #[rstest]
    #[case(
        "0s5256ea13dc110785bddd6694107ef33709369c7f",
        "",
        hex!("f67cde209447349a415c265788d36ddebb3af9b97f0502d19b387e67e4c80eeb318fff299f4b171791a4ae05e112fba0638e4448af4368b6b5f47863982efcea"),
    )]
    #[case(
        "test",
        "test",
        hex!("cb77c650623eb1438b8143e2f2624be75cd8cc5419f2fe5cf3ff42d1dc4764b9c714d12cc6e61e14cc35345e3f9f3bacd6088b94b3f84aed3f1a9184327f4d8b"),
    )]
    #[case(
        "predecessor",
        "path",
        hex!("2da418479003596150f9c58687f5244ba103cf913b1afe5d41b863aefdfd7c00bc24d18e1eb6e9db71ea8d9cb76efe2df2594df304ef23a2847ae1d021768898"),
    )]
    fn check_derivation(
        #[case] predecessor_id: &'static str,
        #[case] path: &str,
        #[case] expected_derived_pk: PublicKey,
    ) {
        let master_pk = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
            &SECP256K1_MPC_PK.into(),
        ))
        .unwrap();
        let predecessor_id = AccountIdRef::new(predecessor_id).unwrap();

        let schema = NearMpcDerivation::<Secp256k1>::new(predecessor_id);
        let tweak = schema.derive_path(path);
        let derived_pk = Secp256k1::derive_public_key(&master_pk, &tweak);

        assert_eq!(
            &derived_pk.to_encoded_point(false).as_bytes()[1..],
            &expected_derived_pk,
            "derived public key mismatch"
        );
    }
}
