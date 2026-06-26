use defuse_digest::{Digest as _, sha3::Sha3_256};
use defuse_kdf::digest::Digest;
use near_account_id::AccountIdRef;

use crate::derive_from_path;

pub type CkdSchema = Digest<Sha3_256>;

/// Prepare [`Schema`](defuse_kdf::Schema) for MPC CKD `AppId` derivation.
///
/// ```rust
/// # use defuse_kdf_mpc::{ckd, kdf::Schema};
/// # use hex_literal::hex;
/// # use near_account_id::AccountIdRef;
/// let predecessor_id = AccountIdRef::new_or_panic("predecessor.near");
///
/// assert_eq!(
///     ckd(predecessor_id).derive_path("mykey"),
///     hex!("7bd78cf4f92b146e3781b3ef3c37f00352e0127d643136291d9992d052524afe"),
/// );
/// ```
pub fn ckd(predecessor_id: impl AsRef<AccountIdRef>) -> CkdSchema {
    // See <https://github.com/near/mpc/blob/f07b9145b17e2372be768aa67a2106be9989a7d7/crates/near-mpc-crypto-types/src/kdf.rs#L15-L23>
    const APP_ID_DERIVATION_PREFIX: &str = "near-mpc v0.1.0 app_id derivation:";

    // TODO: cfg(near)
    thread_local! {
        // per-thread lazily-initialized hasher with pre-processed prefix
        static HASHER: Sha3_256 = Sha3_256::new_with_prefix(APP_ID_DERIVATION_PREFIX);
    }

    derive_from_path(HASHER.with(Clone::clone), predecessor_id)
}

#[cfg(test)]
mod tests {
    use defuse_kdf::Schema;
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    // See <https://github.com/near/mpc/blob/cf179467124203b0187ef0e80b429885b9a51627/crates/near-mpc-crypto-types/src/kdf.rs#L70-L92>
    #[rstest]
    #[case(
        "dwefqwg",
        "frwewegwegweg",
        hex!("b8177b00468f97c337b3f921fbef7aaa959e9a49005aaa59768b795152f4c989"),
    )]
    #[case(
        "dwefqwg",
        "fwei2.3f230",
        hex!("16ac7db8b7c3ad2c5e8fe7d1a9e91ca6634cf9baf84952f85795f515e29cd275"),
    )]
    #[case(
        "dwefqwg",
        "f23fjwef8232",
        hex!("f1d47585609f34fd18fc109340e78c0822c9a010fae30b7e1ca83df48c994986"),
    )]
    #[case(
        "dwefqwg", 
        "fwefwo23fewfw",
        hex!("55591d35775cabb829df65ae7dbd415554a7690657962952555fabf9eb8462e1"),
    )]
    #[case(
        "qfweqwgwegqw",
        "frwewegwegweg",
        hex!("2116717ccc7b1fd0053d96c975d6d2d39321012ff2cf66a328be6707f5d0f48c"),
    )]
    #[case(
        "qfweqwgwegqw",
        "fwei2.3f230",
        hex!("6042862796895d3b0e5678f5bc2da01b245fb8bf7e3515d261a7e93a963e25e3"),
    )]
    #[case(
        "qfweqwgwegqw",
        "f23fjwef8232",
        hex!("344162da86554a281123d58406ce4ba28518113fe1d66405924c9e9b1545e012"),
    )]
    #[case(
        "qfweqwgwegqw",
        "fwefwo23fewfw",
        hex!("90aae886b2533bd33464d922d3ccd1ed1a4d05786b39598ba79a04d0186bd2a4"),
    )]
    #[case(
        "fqwerijqw385",
        "frwewegwegweg",
        hex!("c26434220286c5878fac6d374492b0be520d80f08bdb2504126360e09ba082dc"),
    )]
    #[case(
        "fqwerijqw385",
        "fwei2.3f230",
        hex!("5ef85af87983f28e2b0bc1f2a5ce4d17c8814f86959bb4e1f2d288eafe9c337a"),
    )]
    #[case(
        "fqwerijqw385",
        "f23fjwef8232",
        hex!("b6c7c1cde6185ddb26eb497a2742eac4a05107ce687deaf44ef409c8ba483312"),
    )]
    #[case(
        "fqwerijqw385",
        "fwefwo23fewfw",
        hex!("086cb2e83885d66cd034010d7944e8b29e616be0f378eefa902672898df7b90c"),
    )]
    #[case(
        "fnwef0942534",
        "frwewegwegweg",
        hex!("fe4c1e1948e2716f208ed94d63fa528181d8224234d0f5a22850b58580f84940"),
    )]
    #[case(
        "fnwef0942534",
        "fwei2.3f230",
        hex!("55bd75937a299f9adebd9e12c095fd3479085b097c1456b393cc35a2909b1498"),
    )]
    #[case(
        "fnwef0942534",
        "f23fjwef8232",
        hex!("82b56d55c9876ed6bf79af7a4a7bd21187aa16731d4c033bdb81921d698c6a5d"),
    )]
    #[case(
        "fnwef0942534",
        "fwefwo23fewfw",
        hex!("411fbc46a494fe9a2f8c9cc9c82baee1a634df9578af381901561541ad6f32f3"),
    )]
    fn app_id_has_not_changed(
        #[case] predecessor_id: &str,
        #[case] path: &str,
        #[case] app_id: [u8; 32],
    ) {
        let schema = ckd(AccountIdRef::new_or_panic(predecessor_id));

        assert_eq!(
            schema.derive_path(path),
            app_id,
            "derived app_id has changed"
        );
    }
}
