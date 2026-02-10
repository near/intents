use near_sdk::{AccountId, near};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh(use_discriminant = true), json])]
#[serde(tag = "op", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum WalletOp {
    SetSignatureMode(SetSignatureModeOp) = 0,
    AddExtension(AddExtensionOp) = 1,
    RemoveExtension(RemoveExtensionOp) = 2,

    /// Custom op for third-party implementations
    Custom(CustomOp) = u8::MAX,
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct SetSignatureModeOp {
    pub enable: bool,
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AddExtensionOp {
    pub account_id: AccountId,
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RemoveExtensionOp {
    pub account_id: AccountId,
}

/// Custom op for third-party implementations
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CustomOp {
    pub args: String,
}
