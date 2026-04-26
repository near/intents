use std::borrow::Cow;

use crate::AppId;

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    cfg_attr(feature = "abi", derive(schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Total **non-hierarchical** derivation path
pub struct DerivationPath<'a> {
    /// Identifier of an application to derive for
    pub app_id: AppId<'a>,
    /// Application-specific path
    pub path: Cow<'a, str>,
}

#[cfg(feature = "digest")]
const _: () = {
    use digest::{Digest, Output, Update};
    use digest_io::IoWrapper;
    use sha3::Sha3_256;

    impl DerivationPath<'_> {
        // TODO: do we need other domain separators?
        const PREFIX: &'static [u8] = b"outlayer v0.1.0 tweak derivaton:";

        pub fn hash(&self) -> [u8; 32] {
            // use SHA-3 family as an additional safety measure to prevent from
            // length extension attacks, despite borsh alone would suffice, too
            self.digest::<Sha3_256>().into()
        }

        fn digest<D>(&self) -> Output<D>
        where
            D: Digest + Update,
        {
            let mut hasher = IoWrapper(D::new_with_prefix(Self::PREFIX));
            borsh::to_writer(&mut hasher, self).expect("borsh");
            hasher.0.finalize()
        }
    }
};

#[cfg(all(test, feature = "digest"))]
mod tests {
    use near_account_id::AccountIdRef;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        DerivationPath {
            app_id: AccountIdRef::new_or_panic("test.near").into(),
            path: "".into()
        },
        "7beed0108170c657c97e189db048b402226b2681d427b36e1e62c1984985558b",
    )]
    #[case(
        DerivationPath {
            app_id: AccountIdRef::new_or_panic("test.near").into(),
            path: "test".into()
        },
        "fe502ff6b7cf154385169e4901b745739b2a0327cf2f80024535b3dd023abfc9",
    )]
    #[case(
        DerivationPath {
            app_id: AccountIdRef::new_or_panic("0s1234567890abcdef1234567890abcdef12345678").into(),
            path: "test".into()
        },
        "6f719da726eea8a6bd79f28fffea8f34c404278a6935a0a0c930e8464c91fd9b",
    )]
    fn derive_has_not_changed(#[case] path: DerivationPath<'_>, #[case] hash: &str) {
        let got = hex::encode(path.hash());
        assert_eq!(got, hash, "derived hash has changed");
    }
}
