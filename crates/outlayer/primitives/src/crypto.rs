use crate::AppId;

#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize),
    // cfg_attr(feature = "abi", derive(borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    cfg_attr(feature = "abi", derive(schemars::JsonSchema))
)]
pub struct DerivationPath<'a> {
    pub app_id: AppId<'a>,
    pub path: &'a str,
}
