#[cfg(feature = "contract")]
mod contract;

use core::{
    fmt::{self, Display},
    str::FromStr,
};

use defuse_wallet::{RequestMessage, SignatureSchema};

/// [`SignatureSchema`] which always rejects the signature.
///
/// This can be useful to deploy "1-of-M multisig"/"fan-out" wallet, where
/// extensions are defined at the initialization stage (i.e. `state_init`).
/// So only extensions can execute requests via `w_execute_extension()`.
pub struct NoSign;

impl SignatureSchema for NoSign {
    type PublicKey = NoPublicKey;

    #[inline]
    fn verify(_public_key: &Self::PublicKey, _msg: &RequestMessage, _proof: &str) -> bool {
        false
    }
}

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoPublicKey;

impl Display for NoPublicKey {
    #[inline]
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl FromStr for NoPublicKey {
    type Err = NotEmptyError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.is_empty().then_some(Self).ok_or(NotEmptyError)
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("must be empty")]
pub struct NotEmptyError;
