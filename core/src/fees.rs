use std::borrow::Cow;

pub use defuse_fees::Pips;

use near_sdk::{AccountId, AccountIdRef, near};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct FeesConfig {
    pub fee: Pips,
    pub fee_collector: AccountId,
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FeeChangedEvent {
    pub old_fee: Pips,
    pub new_fee: Pips,
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FeeCollectorChangedEvent<'a> {
    pub old_fee_collector: Cow<'a, AccountIdRef>,
    pub new_fee_collector: Cow<'a, AccountIdRef>,
}
