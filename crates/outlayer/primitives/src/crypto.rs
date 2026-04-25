use std::borrow::Cow;

use crate::AppId;

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    cfg_attr(feature = "abi", derive(schemars::JsonSchema))
)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DerivationPath<'a> {
    /// Identifier of an application to derive for
    pub app_id: AppId<'a>,
    /// Application-specific path
    pub path: Cow<'a, str>,
}
