use near_sdk::{AccountId, near};

#[near(serializers = [borsh, json])]
#[serde(tag = "op", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletOp {
    SetSignatureMode {
        enable: bool,
    },
    AddExtension {
        // TODO: check not self?
        // TODO: but was if it originally was in state_init?
        // this can never happen due to recursive hashing
        account_id: AccountId,
    },
    RemoveExtension {
        account_id: AccountId,
    },
}
