use std::borrow::Cow;

pub use near_account_id::*;

#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    cfg_attr(feature = "abi", derive(schemars::JsonSchema))
)]
#[repr(u8)]
pub enum AppId<'a> {
    Near(Cow<'a, AccountIdRef>),
}

impl<'a> AppId<'a> {
    pub fn near(account_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self::Near(account_id.into())
    }

    pub fn as_ref<'b: 'a>(&'b self) -> AppId<'b> {
        match self {
            Self::Near(app_id) => Self::Near(Cow::Borrowed(&app_id)),
        }
    }
}

impl From<AccountId> for AppId<'_> {
    fn from(account_id: AccountId) -> Self {
        Self::Near(Cow::Owned(account_id))
    }
}

impl<'a> From<&'a AccountIdRef> for AppId<'a> {
    fn from(account_id: &'a AccountIdRef) -> Self {
        Self::Near(Cow::Borrowed(account_id))
    }
}
