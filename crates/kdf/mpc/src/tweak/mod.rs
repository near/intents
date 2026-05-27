#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;

use core::marker::PhantomData;

use defuse_kdf::{CurveArithmetic, Derive, DeriveExt, Schema, digest::Digest};
use impl_tools::autoimpl;
use near_account_id::AccountIdRef;
use sha3::{Digest as _, Sha3_256};

use crate::derive_from_path;

pub type TweakSchema<C> = Derive<ToScalar<C>, Digest<Sha3_256>>;

/// Prepare [`Schema`](defuse_kdf::Schema) for MPC tweak derivation.
pub fn tweak<C>(predecessor_id: impl AsRef<AccountIdRef>) -> TweakSchema<C>
where
    C: NearMpcCurve,
{
    ToScalar::<C>::new().derive(derive_tweak(predecessor_id))
}

fn derive_tweak(predecessor_id: impl AsRef<AccountIdRef>) -> Digest<Sha3_256> {
    // See <https://github.com/near/mpc/blob/f07b9145b17e2372be768aa67a2106be9989a7d7/crates/near-mpc-crypto-types/src/kdf.rs#L6-L13>
    const TWEAK_DERIVATION_PREFIX: &str = "near-mpc-recovery v0.1.0 epsilon derivation:";

    thread_local! {
        // per-thread lazily-initialized hasher with pre-processed prefix
        static HASHER: Sha3_256 = Sha3_256::new_with_prefix(TWEAK_DERIVATION_PREFIX);
    }

    derive_from_path(HASHER.with(Clone::clone), predecessor_id)
}

#[autoimpl(Debug, Clone, Copy, Default)]
pub struct ToScalar<C>(PhantomData<C>);

impl<C> ToScalar<C> {
    #[inline]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<C> Schema<[u8; 32]> for ToScalar<C>
where
    C: NearMpcCurve,
{
    type Output = C::Scalar;

    fn derive_path(&self, tweak: [u8; 32]) -> Self::Output {
        C::to_scalar(tweak)
    }
}

pub trait NearMpcCurve: CurveArithmetic + sealed::Sealed {
    fn to_scalar(tweak: [u8; 32]) -> Self::Scalar;
}

mod sealed {
    pub trait Sealed {}
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    // See <https://github.com/near/mpc/blob/cf179467124203b0187ef0e80b429885b9a51627/crates/near-mpc-crypto-types/src/kdf.rs#L46-L68>
    #[rstest]
    #[case(
        "dwefqwg",
        "frwewegwegweg",
        hex!("ad2d6fc1f445a1b43830415e7e6e3d03cd077673047848a085f130c24f53ee00"),
    )]
    #[case(
        "dwefqwg",
        "fwei2.3f230",
        hex!("5f76eba48a20e3dd355e38f512c142ab8c8da4999f6b32ecd326d227b44ac05a"),
    )]
    #[case(
        "dwefqwg",
        "f23fjwef8232",
        hex!("d133f73abcb3c7cb944dcc0dd3bf72dd89fdc3cf988c9f025af571a205c9b730"),
    )]
    #[case(
        "dwefqwg", 
        "fwefwo23fewfw",
        hex!("3dd0192c45be59eee3dd0ba62776864245f7aac29514314b80baab2057453ec7"),
    )]
    #[case(
        "qfweqwgwegqw",
        "frwewegwegweg",
        hex!("510cd39f09b3c1bdc1fd655ce57ab29eb20dfce647584727fcca0e483163a8d0"),
    )]
    #[case(
        "qfweqwgwegqw",
        "fwei2.3f230",
        hex!("9cf7f40ab14bc5dd6feee48122b3d85f58461bd595e436fac186b8fa384f3048"),
    )]
    #[case(
        "qfweqwgwegqw",
        "f23fjwef8232",
        hex!("f53f5cb5c6902ee079681013b452f1908922f48fd668ad6ca522bd49842bf72a"),
    )]
    #[case(
        "qfweqwgwegqw",
        "fwefwo23fewfw",
        hex!("79051ad14a62552ee9da3fb813813a9d2f2fa8e1549e5f4c658254052562e18f"),
    )]
    #[case(
        "fqwerijqw385",
        "frwewegwegweg",
        hex!("f91edb212a03607ab61ce79f54e38901af6d9ff8709593f2d303e590691c26a6"),
    )]
    #[case(
        "fqwerijqw385",
        "fwei2.3f230",
        hex!("dfb60b3ba4348d3d787aa1af912910a4ccc2fb34e92c010fdae6730f4fab0734"),
    )]
    #[case(
        "fqwerijqw385",
        "f23fjwef8232",
        hex!("353862e7968f7f058a4f0498afbbcd504b6621690dfb3118878a7a6502e22e0e"),
    )]
    #[case(
        "fqwerijqw385",
        "fwefwo23fewfw",
        hex!("ac05f6320b0ed20f9f18a3531c571db237f41ab9b31203598bd8239631cb040f"),
    )]
    #[case(
        "fnwef0942534",
        "frwewegwegweg",
        hex!("7571fcaad0445387087ebcde589540e80926cfd08144aa3f45bc78b8683d06c3"),
    )]
    #[case(
        "fnwef0942534",
        "fwei2.3f230",
        hex!("74f118cfd013dc109848cfc5e0f963e1781dcbb095ae6429623007ccf59b6248"),
    )]
    #[case(
        "fnwef0942534",
        "f23fjwef8232",
        hex!("874bdff117d6d90c0eb5cd7a181e7f20d99abfaa04f503e97867ec08f6796f96"),
    )]
    #[case(
        "fnwef0942534",
        "fwefwo23fewfw",
        hex!("91da4d8973ea3ccaad9c7b71cb81fa512c4cc939c5157cdbf6a9605d3d1ce4a2"),
    )]
    fn tweak_has_not_changed(
        #[case] predecessor_id: &str,
        #[case] path: &str,
        #[case] tweak: [u8; 32],
    ) {
        let schema = derive_tweak(AccountIdRef::new_or_panic(predecessor_id));

        assert_eq!(schema.derive_path(path), tweak, "derived tweak has changed")
    }
}
