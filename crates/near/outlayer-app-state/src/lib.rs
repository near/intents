#[cfg(feature = "borsh")]
use std::collections::BTreeMap;

pub use near_account_id::AccountId;

#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub admin_id: AccountId,
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
    pub code_hash: [u8; 32],
    pub code_url: String,
}

impl State {
    pub const STATE_KEY: &[u8] = b"";

    pub fn new(admin_id: impl Into<AccountId>, code_hash: [u8; 32], code_url: String) -> Self {
        Self {
            admin_id: admin_id.into(),
            code_hash,
            code_url,
        }
    }

    #[cfg(feature = "borsh")]
    pub fn state_init(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            ::borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
