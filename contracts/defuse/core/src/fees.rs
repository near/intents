use std::borrow::Cow;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::{AccountId, AccountIdRef};

pub use defuse_fees::{Pips, PipsOutOfRange};

#[cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct FeesConfig {
    pub fee: Pips,
    pub fee_collector: AccountId,
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeChangedEvent {
    pub old_fee: Pips,
    pub new_fee: Pips,
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCollectorChangedEvent<'a> {
    pub old_fee_collector: Cow<'a, AccountIdRef>,
    pub new_fee_collector: Cow<'a, AccountIdRef>,
}
