use std::borrow::Cow;

use borsh::{BorshDeserialize, BorshSerialize};
pub use defuse_fees::{Pips, PipsOutOfRange};
use near_sdk::{AccountId, AccountIdRef};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema, ::borsh::BorshSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct FeesConfig {
    pub fee: Pips,
    pub fee_collector: AccountId,
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeChangedEvent {
    pub old_fee: Pips,
    pub new_fee: Pips,
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCollectorChangedEvent<'a> {
    pub old_fee_collector: Cow<'a, AccountIdRef>,
    pub new_fee_collector: Cow<'a, AccountIdRef>,
}
