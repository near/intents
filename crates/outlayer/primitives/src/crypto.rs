use std::borrow::Cow;

use crate::AppId;

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
    use digest::{Output, Update};
    use digest_io::IoWrapper;
    use sha3::{Digest, Sha3_256};

    impl DerivationPath<'_> {
        // TODO: do we need other domain separators?
        const PREFIX: &'static [u8] = b"outlayer v0.1.0 tweak derivaton:";

        pub fn hash(&self) -> [u8; 32] {
            // we use SHA-3 family as an additional safety measure to prevent
            // from length extension attacks, despite borsh alone would suffice
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
