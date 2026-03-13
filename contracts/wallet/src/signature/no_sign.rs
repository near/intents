use core::{
    fmt::{self, Display},
    str::FromStr,
};

use near_sdk::{
    near,
    serde_with::{DeserializeFromStr, SerializeDisplay},
};
use thiserror::Error as ThisError;

use crate::signature::SigningStandard;

/// [`SigningStandard`] which always rejects the signature.
///
/// This can be useful to deploy "1-of-M multisig"/"fan-out" wallet, where
/// extensions are defined at the initialization stage (i.e. `state_init`).
/// So only extensions can execute requests via `w_execute_extension()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoSign;

impl<M> SigningStandard<M> for NoSign {
    type PublicKey = NoPublicKey;

    fn verify(_msg: M, _public_key: &Self::PublicKey, _signature: &str) -> bool {
        false
    }
}

/// [`SigningStandard::PublicKey`] for `NoSign`
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    derive(near_sdk::schemars::JsonSchema),
    schemars(crate = "::near_sdk::schemars", with = "String")
)]
#[near(serializers = [borsh])]
#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeDisplay, DeserializeFromStr)]
#[serde_with(crate = "::near_sdk::serde_with")]
pub struct NoPublicKey;

impl Display for NoPublicKey {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl FromStr for NoPublicKey {
    type Err = NotEmptyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.is_empty().then_some(Self).ok_or(NotEmptyError)
    }
}

#[derive(Debug, Clone, Copy, ThisError, PartialEq, Eq)]
#[error("must be empty")]
pub struct NotEmptyError;
