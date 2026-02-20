use near_sdk::{AccountId, near, serde_with::base64::Base64};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh(use_discriminant = true), json])]
#[serde(tag = "op", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum WalletOp {
    SetSignatureMode {
        enable: bool,
    } = 0,
    AddExtension {
        account_id: AccountId,
    } = 1,
    RemoveExtension {
        account_id: AccountId,
    } = 2,

    /// Custom op for third-party implementations.
    /// TODO: do we even this variant? Or custom implementations
    /// can just add their own wariants?
    Custom {
        #[cfg_attr(
            all(feature = "abi", not(target_arch = "wasm32")),
            schemars(with = "String")
        )]
        #[serde_as(as = "Base64")]
        args: Vec<u8>,
    } = u8::MAX,
}
