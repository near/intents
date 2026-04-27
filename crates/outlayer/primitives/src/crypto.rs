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
        const PREFIX: &'static [u8] = b"outlayer v0.1.0 tweak derivation:";

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
    use hex_literal::hex;
    use near_account_id::AccountIdRef;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        DerivationPath {
            app_id: AccountIdRef::new_or_panic("test.near").into(),
            path: "".into()
        },
        hex!("19662bc997912779e2f54550bb06b07006943eb106af0632a669e350d8faa245"),
    )]
    #[case(
        DerivationPath {
            app_id: AccountIdRef::new_or_panic("test.near").into(),
            path: "test".into()
        },
        hex!("b581f6b4c6b43a673777747ea01d69891342ebd35625e927d3f12403631c33fb"),
    )]
    #[case(
        DerivationPath {
            app_id: AccountIdRef::new_or_panic("0s1234567890abcdef1234567890abcdef12345678").into(),
            path: "test".into()
        },
        hex!("a1c48b73cf43f80611edeae5e1f809b776af00adde05195984573c2ab22c395f"),
    )]
    fn derive_has_not_changed(#[case] path: DerivationPath<'_>, #[case] hash: [u8; 32]) {
        assert_eq!(path.hash(), hash, "derived hash has changed");
    }
}
