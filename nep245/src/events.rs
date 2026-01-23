use super::TokenId;
use derive_more::derive::From;
use near_sdk::{
    AccountIdRef, AsNep297Event, FunctionError, json_types::U128, near, serde::Deserialize,
};
use std::borrow::Cow;

/// Error returned when a refund log would exceed the maximum allowed length.
#[derive(Debug, Clone, PartialEq, Eq, FunctionError, thiserror::Error)]
#[error("Event log is too long: {log_length} bytes exceeds limit of {limit} bytes")]
pub struct ErrorRefundLogTooLong {
    pub log_length: usize,
    pub limit: usize,
}

/// NEAR protocol limit for log messages (16 KiB)
pub const TOTAL_LOG_LENGTH_LIMIT: usize = 16384;

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "nep245"))]
#[derive(Debug, Clone, Deserialize, From)]
pub enum MtEvent<'a> {
    #[event_version("1.0.0")]
    MtMint(Cow<'a, [MtMintEvent<'a>]>),
    #[event_version("1.0.0")]
    MtBurn(Cow<'a, [MtBurnEvent<'a>]>),
    #[event_version("1.0.0")]
    MtTransfer(Cow<'a, [MtTransferEvent<'a>]>),
}

pub trait EmitChecked {
    fn emit_with_refund_log_checked(self) -> Result<(), ErrorRefundLogTooLong>;
}

const REFUND_EXTRA_BYTES: usize = r#","memo":"refund""#.len();
const REFUND_STR_LEN: usize = "refund".len();

fn refund_log_extra_bytes_count(memo: Option<&str>) -> usize {
    memo.map_or(REFUND_EXTRA_BYTES, |m| {
        REFUND_STR_LEN.saturating_sub(m.len())
    })
}

fn compute_refund_overhead(event: &MtEvent<'_>) -> usize {
    match event {
        MtEvent::MtMint(events) => events
            .iter()
            .map(|e| refund_log_extra_bytes_count(e.memo.as_deref()))
            .fold(0, usize::saturating_add),
        MtEvent::MtBurn(events) => events
            .iter()
            .map(|e| refund_log_extra_bytes_count(e.memo.as_deref()))
            .fold(0, usize::saturating_add),
        MtEvent::MtTransfer(events) => events
            .iter()
            .map(|e| refund_log_extra_bytes_count(e.memo.as_deref()))
            .fold(0, usize::saturating_add),
    }
}

impl MtEvent<'_> {
    /// Validates that the event log (including potential refund overhead) fits within limits.
    /// Returns the log string to emit on success.
    fn validate_with_refund_overhead(&self) -> Result<String, ErrorRefundLogTooLong> {
        let log = self.to_nep297_event().to_event_log();
        let overhead = compute_refund_overhead(self);
        let total = log.len() + overhead;

        if total > TOTAL_LOG_LENGTH_LIMIT {
            return Err(ErrorRefundLogTooLong {
                log_length: total,
                limit: TOTAL_LOG_LENGTH_LIMIT,
            });
        }
        Ok(log)
    }
}

impl<'a, T> EmitChecked for T
where
    T: Into<MtEvent<'a>>,
{
    fn emit_with_refund_log_checked(self) -> Result<(), ErrorRefundLogTooLong> {
        let event = self.into();
        let log = event.validate_with_refund_overhead()?;
        near_sdk::env::log_str(&log);
        Ok(())
    }
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct MtMintEvent<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct MtBurnEvent<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    pub token_ids: Cow<'a, [TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct MtTransferEvent<'a> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    pub old_owner_id: Cow<'a, AccountIdRef>,
    pub new_owner_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

impl MtTransferEvent<'_> {
    /// Calculate the size of a refund event log for this transfer.
    /// Creates a new event with swapped owner IDs and "refund" memo.
    #[must_use]
    pub fn refund_log_size(&self) -> usize {
        MtEvent::MtTransfer(
            [MtTransferEvent {
                authorized_id: None,
                old_owner_id: self.new_owner_id.clone(),
                new_owner_id: self.old_owner_id.clone(),
                token_ids: self.token_ids.clone(),
                amounts: self.amounts.clone(),
                memo: Some("refund".into()),
            }]
            .as_slice()
            .into(),
        )
        .to_nep297_event()
        .to_event_log()
        .len()
    }
}

/// A trait that's used to make it possible to call `emit()` on the enum
/// arms' contents without having to explicitly construct the enum `MtEvent` itself
pub trait MtEventEmit<'a>: Into<MtEvent<'a>> {
    #[inline]
    fn emit(self) {
        MtEvent::emit(&self.into());
    }
}
impl<'a, T> MtEventEmit<'a> for T where T: Into<MtEvent<'a>> {}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::json_types::U128;

    /// Create a single-event `MtTransfer` with exact log length.
    /// Pads `token_id` to achieve the desired length.
    fn create_single_event_mt(length: usize, memo: Option<&str>) -> MtEvent<'static> {
        let old_owner: near_sdk::AccountId = "aa".parse().unwrap();
        let new_owner: near_sdk::AccountId = "bb".parse().unwrap();
        let base_token_id = "t";

        // Measure base log length
        let base_event = MtTransferEvent {
            authorized_id: None,
            old_owner_id: Cow::Owned(old_owner.clone()),
            new_owner_id: Cow::Owned(new_owner.clone()),
            token_ids: Cow::Owned(vec![base_token_id.to_string()]),
            amounts: Cow::Owned(vec![U128(1)]),
            memo: memo.map(|m| Cow::Owned(m.to_string())),
        };
        let base_mt_event = MtEvent::MtTransfer(Cow::Owned(vec![base_event]));
        let base_length = base_mt_event.to_nep297_event().to_event_log().len();

        // Calculate padding needed for token_id
        let padding_needed = length.saturating_sub(base_length);
        let padded_token_id = format!("{}{}", base_token_id, "x".repeat(padding_needed));

        let event = MtTransferEvent {
            authorized_id: None,
            old_owner_id: Cow::Owned(old_owner),
            new_owner_id: Cow::Owned(new_owner),
            token_ids: Cow::Owned(vec![padded_token_id]),
            amounts: Cow::Owned(vec![U128(1)]),
            memo: memo.map(|m| Cow::Owned(m.to_string())),
        };

        let mt_event = MtEvent::MtTransfer(Cow::Owned(vec![event]));
        let log_len = mt_event.to_nep297_event().to_event_log().len();
        assert_eq!(
            log_len, length,
            "Expected log length {length}, got {log_len}"
        );

        mt_event
    }

    /// Create a triple-event `MtTransfer` with exact log length.
    /// Each event has the same memo. Pads first event's `token_id` to achieve the desired length.
    fn create_triple_event_mt(length: usize, memo: Option<&str>) -> MtEvent<'static> {
        let old_owner: near_sdk::AccountId = "aa".parse().unwrap();
        let new_owner: near_sdk::AccountId = "bb".parse().unwrap();
        let base_token_id = "t";

        // Measure base log length with 3 events
        let base_events: Vec<MtTransferEvent<'static>> = (0..3)
            .map(|i| MtTransferEvent {
                authorized_id: None,
                old_owner_id: Cow::Owned(old_owner.clone()),
                new_owner_id: Cow::Owned(new_owner.clone()),
                token_ids: Cow::Owned(vec![format!("{base_token_id}{i}")]),
                amounts: Cow::Owned(vec![U128(1)]),
                memo: memo.map(|m| Cow::Owned(m.to_string())),
            })
            .collect();
        let base_mt_event = MtEvent::MtTransfer(Cow::Owned(base_events));
        let base_length = base_mt_event.to_nep297_event().to_event_log().len();

        // Calculate padding needed (only pad the first event's token_id)
        let padding_needed = length.saturating_sub(base_length);
        let padded_token_id = format!("{base_token_id}0{}", "x".repeat(padding_needed));

        // Create final events: first one with padded token_id, rest with base token_ids
        let events: Vec<MtTransferEvent<'static>> = (0..3)
            .map(|i| {
                let token_id = if i == 0 {
                    padded_token_id.clone()
                } else {
                    format!("{base_token_id}{i}")
                };
                MtTransferEvent {
                    authorized_id: None,
                    old_owner_id: Cow::Owned(old_owner.clone()),
                    new_owner_id: Cow::Owned(new_owner.clone()),
                    token_ids: Cow::Owned(vec![token_id]),
                    amounts: Cow::Owned(vec![U128(1)]),
                    memo: memo.map(|m| Cow::Owned(m.to_string())),
                }
            })
            .collect();

        let mt_event = MtEvent::MtTransfer(Cow::Owned(events));
        let log_len = mt_event.to_nep297_event().to_event_log().len();
        assert_eq!(
            log_len, length,
            "Expected log length {length}, got {log_len}"
        );

        mt_event
    }

    #[test]
    fn single_event_no_memo_at_limit_minus_overhead_passes() {
        let mt = create_single_event_mt(TOTAL_LOG_LENGTH_LIMIT - REFUND_EXTRA_BYTES, None);
        assert!(mt.validate_with_refund_overhead().is_ok());
    }

    #[test]
    fn single_event_short_memo_at_limit_fails() {
        let mt = create_single_event_mt(TOTAL_LOG_LENGTH_LIMIT, Some("refu"));
        let err = mt.validate_with_refund_overhead().unwrap_err();
        // With memo "refu" (4 chars), overhead is REFUND_STR_LEN - 4 = 2
        assert_eq!(
            err.log_length,
            TOTAL_LOG_LENGTH_LIMIT + REFUND_STR_LEN - "refu".len()
        );
    }

    #[test]
    fn single_event_over_limit_fails() {
        let mt = create_single_event_mt(TOTAL_LOG_LENGTH_LIMIT + 1, Some("refund1"));
        assert!(mt.validate_with_refund_overhead().is_err());
    }

    #[test]
    fn triple_event_no_memo_at_limit_minus_overhead_passes() {
        let mt = create_triple_event_mt(TOTAL_LOG_LENGTH_LIMIT - 3 * REFUND_EXTRA_BYTES, None);
        assert!(mt.validate_with_refund_overhead().is_ok());
    }

    #[test]
    fn triple_event_short_memo_at_limit_fails() {
        let mt = create_triple_event_mt(TOTAL_LOG_LENGTH_LIMIT, Some("refu"));
        let err = mt.validate_with_refund_overhead().unwrap_err();
        // With memo "refu" (4 chars), overhead per event is REFUND_STR_LEN - 4 = 2
        assert_eq!(
            err.log_length,
            TOTAL_LOG_LENGTH_LIMIT + 3 * (REFUND_STR_LEN - "refu".len())
        );
    }

    #[test]
    fn triple_event_over_limit_fails() {
        let mt = create_triple_event_mt(TOTAL_LOG_LENGTH_LIMIT + 1, Some("refund1"));
        assert!(mt.validate_with_refund_overhead().is_err());
    }
}
