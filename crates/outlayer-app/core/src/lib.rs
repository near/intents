use std::{borrow::Cow, collections::BTreeMap};

use near_account_id::AccountIdRef;

mod public_key;
pub use public_key::AdminPublicKey;
mod state;
pub use state::IMMUTABLE_ADMIN_ID;

#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
/// State of an Outlayer App contract
#[cfg_attr(feature = "parse", derive(Debug))]
#[derive(Clone, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct State<'a> {
    pub admin_id: Cow<'a, AccountIdRef>,
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
    pub code_hash: [u8; 32],
    pub code_url: Cow<'a, str>,
    pub admin_public_key: AdminPublicKey,
    pub state: BTreeMap<Vec<u8>, Vec<u8>>,
    pub config: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl State<'_> {
    pub const STATE_KEY: &'static [u8] = b"";

    #[cfg(feature = "borsh")]
    pub fn state_init(&self) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            ::borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
