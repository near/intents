use std::borrow::Cow;

use near_account_id::AccountIdRef;

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
pub struct State<'a> {
    pub admin_id: Cow<'a, AccountIdRef>,
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
    pub code_hash: [u8; 32],
    pub code_url: Cow<'a, str>,
}

impl<'a> State<'a> {
    pub const STATE_KEY: &'static [u8] = b"";

    pub fn new(
        admin_id: impl Into<Cow<'a, AccountIdRef>>,
        code_hash: [u8; 32],
        code_url: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            admin_id: admin_id.into(),
            code_hash,
            code_url: code_url.into(),
        }
    }

    #[cfg(feature = "borsh")]
    pub fn state_init(&self) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            ::borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
