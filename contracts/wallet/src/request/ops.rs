use near_sdk::{AccountId, near};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh(use_discriminant = true), json])]
#[serde(tag = "op", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum WalletOp {
    SetSignatureMode { enable: bool } = 0,
    AddExtension { account_id: AccountId } = 1,
    RemoveExtension { account_id: AccountId } = 2,
}
