use std::{
    borrow::Cow,
    fmt::{self, Debug, Display},
    str::FromStr,
};

pub use near_account_id as account_id;
use near_account_id::{AccountId, AccountIdRef, ParseAccountError};
use strum::{EnumDiscriminants, EnumString};

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "abi",
        derive(schemars::JsonSchema),
        schemars(with = "String"),
    )
)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, EnumDiscriminants, derive_more::From)]
#[strum_discriminants(
    doc = "Type of [`AppId`]",
    vis(pub),
    name(AppIdType),
    derive(strum::Display, EnumString),
    strum(serialize_all = "snake_case"),
    cfg_attr(
        feature = "serde",
        derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr),
        cfg_attr(
            feature = "abi",
            derive(schemars::JsonSchema),
            schemars(with = "String"),
        )
    ),
    cfg_attr(
        feature = "borsh",
        derive(borsh::BorshSerialize, borsh::BorshDeserialize),
        cfg_attr(feature = "abi", derive(borsh::BorshSchema)),
        borsh(use_discriminant = true),
    )
)]
#[repr(u8)]
/// Application identifier
pub enum AppId<'a> {
    #[from(AccountId, &'a AccountIdRef)]
    Near(Cow<'a, AccountIdRef>) = 0,
}

impl<'a> AppId<'a> {
    #[allow(clippy::missing_const_for_fn)]
    #[inline]
    pub fn as_ref<'b: 'a>(&'b self) -> AppId<'b> {
        match self {
            Self::Near(app_id) => Self::Near(Cow::Borrowed(app_id)),
        }
    }
}

impl Debug for AppId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Near(account_id) => write!(f, "{}:{}", AppIdType::Near, account_id),
        }
    }
}

impl Display for AppId<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl FromStr for AppId<'_> {
    type Err = ParseAppIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (typ, data) = s
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        match typ.parse()? {
            AppIdType::Near => data
                .parse::<AccountId>()
                .map(Into::into)
                .map_err(Into::into),
        }
    }
}

/// An error occurred while parsing [`AppId`]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseAppIdError {
    #[error("AccountId: {0}")]
    AccountId(#[from] ParseAccountError),
    #[error(transparent)]
    ParseError(#[from] strum::ParseError),
}
