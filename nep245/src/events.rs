use super::TokenId;
use crate::checked::{ErrorLogTooLong, RefundCheckedMtEvent};
use defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT;
use derive_more::derive::From;
use near_sdk::{AccountIdRef, AsNep297Event, json_types::U128, near, serde::Deserialize};
use std::borrow::Cow;

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

impl MtEvent<'_> {
    /// Validates that the event log (including potential refund overhead) fits within limits.
    /// Returns a [`RefundCheckedMtEvent`] that can be emitted.
    pub fn check_refund(self) -> Result<RefundCheckedMtEvent, ErrorLogTooLong> {
        let log = self.to_nep297_event().to_event_log();
        let delta = self.compute_refund_delta();
        let refund_len = log
            .len()
            .saturating_add(delta.overhead())
            .saturating_sub(delta.savings());

        if refund_len > TOTAL_LOG_LENGTH_LIMIT {
            return Err(ErrorLogTooLong);
        }
        Ok(RefundCheckedMtEvent(log))
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
    use crate::checked::REFUND_EXTRA_BYTES;
    use defuse_near_utils::REFUND_MEMO;

    use super::*;
    use near_sdk::json_types::U128;

    const REFUND_STR_LEN: usize = REFUND_MEMO.len();

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
    /// Each event has its own memo. Pads first event's `token_id` to achieve the desired length.
    fn create_triple_event_mt(length: usize, memos: [Option<&str>; 3]) -> MtEvent<'static> {
        let old_owner: near_sdk::AccountId = "aa".parse().unwrap();
        let new_owner: near_sdk::AccountId = "bb".parse().unwrap();
        let base_token_id = "t";

        // Measure base log length with 3 events
        let base_events: Vec<MtTransferEvent<'static>> = memos
            .iter()
            .enumerate()
            .map(|(i, memo)| MtTransferEvent {
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
        let events: Vec<MtTransferEvent<'static>> = memos
            .iter()
            .enumerate()
            .map(|(i, memo)| {
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
        assert!(mt.check_refund().is_ok());
    }

    #[test]
    fn single_event_short_memo_at_limit_fails() {
        let memo = "refu";
        let mt = create_single_event_mt(TOTAL_LOG_LENGTH_LIMIT, Some(memo));
        assert!(matches!(mt.check_refund().unwrap_err(), ErrorLogTooLong));
    }

    #[test]
    fn triple_event_no_memo_at_limit_minus_overhead_passes() {
        let mt = create_triple_event_mt(TOTAL_LOG_LENGTH_LIMIT - 3 * REFUND_EXTRA_BYTES, [None; 3]);
        assert!(mt.check_refund().is_ok());
    }

    #[test]
    fn triple_event_short_memo_at_limit_fails() {
        let mt = create_triple_event_mt(TOTAL_LOG_LENGTH_LIMIT, [Some("refu"); 3]);
        assert!(matches!(mt.check_refund().unwrap_err(), ErrorLogTooLong));
    }

    #[test]
    fn triple_event_mixed_memos_overhead_equals_savings_at_limit_passes() {
        // there are 3 events
        // 1 without memo
        // 2 with "refund" memo
        // 3 with really long memo
        // total log length is exactly TOTAL_LOG_LENGTH_LIMIT, but since really long memo will be
        // replaced with just refund there will be enough buffer to set memo "refund" also for
        // first event and still fit into TOTAL_LOG_LENGTH_LIMIT on refund
        let long_memo = "x".repeat(REFUND_EXTRA_BYTES + REFUND_STR_LEN);
        assert_eq!(long_memo.len() - REFUND_STR_LEN, REFUND_EXTRA_BYTES);

        let mt = create_triple_event_mt(
            TOTAL_LOG_LENGTH_LIMIT,
            [None, Some("refund"), Some(&long_memo)],
        );
        assert!(mt.check_refund().is_ok());
    }
}
