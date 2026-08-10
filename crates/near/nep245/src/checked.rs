use super::ErrorLogTooLong;
use crate::MtEvent;
use defuse_near_utils::REFUND_MEMO;

const REFUND_STR_LEN: usize = REFUND_MEMO.len();
const REFUND_EXTRA_BYTES: usize = r#","memo":""#.len() + REFUND_STR_LEN;

/// A validated event log that has been checked for refund overhead.
/// Use [`CheckedMtEvent::emit`] to emit the event.
#[derive(Debug)]
#[must_use = "call `.emit()` to emit the event"]
pub struct CheckedMtEvent(pub(crate) String);

impl CheckedMtEvent {
    pub fn emit(self) {
        near_sdk::env::log_str(&self.0);
    }
}

#[derive(Default, Clone, Copy)]
#[must_use]
struct RefundLogDelta {
    overhead: usize,
    savings: usize,
}

impl RefundLogDelta {
    const fn new(overhead: usize, savings: usize) -> Self {
        Self {
            overhead: overhead.saturating_sub(savings),
            savings: savings.saturating_sub(overhead),
        }
    }

    const fn overhead(self) -> usize {
        self.overhead
    }

    const fn savings(self) -> usize {
        self.savings
    }

    const fn saturating_add(self, other: Self) -> Self {
        Self::new(
            self.overhead.saturating_add(other.overhead),
            self.savings.saturating_add(other.savings),
        )
    }
}

const fn refund_log_delta(memo: Option<&str>) -> RefundLogDelta {
    let Some(m) = memo else {
        return RefundLogDelta {
            overhead: REFUND_EXTRA_BYTES,
            savings: 0,
        };
    };
    RefundLogDelta::new(REFUND_STR_LEN, m.len())
}

impl MtEvent<'_> {
    /// Validates that the event log (including potential refund overhead) fits within limits.
    /// Returns a [`CheckedMtEvent`] that can be emitted.
    pub fn check_refund(self) -> Result<CheckedMtEvent, ErrorLogTooLong> {
        use near_sdk::AsNep297Event;

        let log = self.to_nep297_event().to_event_log();
        let delta = self.compute_refund_delta();
        let refund_len = log
            .len()
            .saturating_add(delta.overhead())
            .saturating_sub(delta.savings());

        if refund_len > defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT {
            return Err(ErrorLogTooLong);
        }
        Ok(CheckedMtEvent(log))
    }

    fn compute_refund_delta(&self) -> RefundLogDelta {
        match self {
            MtEvent::MtMint(events) => events
                .iter()
                .map(|e| refund_log_delta(e.memo.as_deref()))
                .fold(RefundLogDelta::default(), RefundLogDelta::saturating_add),
            MtEvent::MtBurn(events) => events
                .iter()
                .map(|e| refund_log_delta(e.memo.as_deref()))
                .fold(RefundLogDelta::default(), RefundLogDelta::saturating_add),
            MtEvent::MtTransfer(events) => events
                .iter()
                .map(|e| refund_log_delta(e.memo.as_deref()))
                .fold(RefundLogDelta::default(), RefundLogDelta::saturating_add),
        }
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use near_account_id::AccountId;
    use near_sdk::AsNep297Event;

    use super::refund_log_delta;
    use crate::MtTransferEvent;
    use crate::checked::{REFUND_EXTRA_BYTES, REFUND_STR_LEN};
    use crate::{ErrorLogTooLong, MtEvent};

    #[test]
    fn test_refund_log_delta_shorter_memo() {
        let delta = refund_log_delta(Some("r"));
        assert_eq!(delta.savings(), 0);
        assert_eq!(delta.overhead(), 5);
    }

    #[test]
    fn test_refund_log_delta_longer_memo() {
        let delta = refund_log_delta(Some("refund123"));
        assert_eq!(delta.savings(), 3);
        assert_eq!(delta.overhead(), 0);
    }

    #[test]
    fn test_refund_log_delta_equal_memo() {
        let delta = refund_log_delta(Some("123456"));
        assert_eq!(delta.savings(), 0);
        assert_eq!(delta.overhead(), 0);
    }

    #[test]
    fn test_refund_log_delta_empty_memo() {
        let delta = refund_log_delta(None);
        assert_eq!(delta.savings(), 0);
        assert_eq!(delta.overhead(), REFUND_EXTRA_BYTES);
    }

    /// Create a single-event `MtTransfer` with exact log length.
    /// Pads `token_id` to achieve the desired length.
    fn create_single_event_mt(length: usize, memo: Option<&str>) -> MtEvent<'static> {
        let old_owner: AccountId = "aa".parse().unwrap();
        let new_owner: AccountId = "bb".parse().unwrap();
        let base_token_id = "t";

        // Measure base log length
        let base_event = MtTransferEvent {
            authorized_id: None,
            old_owner_id: Cow::Owned(old_owner.clone()),
            new_owner_id: Cow::Owned(new_owner.clone()),
            token_ids: Cow::Owned(vec![base_token_id.to_string()]),
            amounts: Cow::Owned(vec![1]),
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
            amounts: Cow::Owned(vec![1]),
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
        let old_owner: AccountId = "aa".parse().unwrap();
        let new_owner: AccountId = "bb".parse().unwrap();
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
                amounts: Cow::Owned(vec![1]),
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
                    amounts: Cow::Owned(vec![1]),
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
        let mt = create_single_event_mt(
            defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT - REFUND_EXTRA_BYTES,
            None,
        );
        assert!(mt.check_refund().is_ok());
    }

    #[test]
    fn single_event_short_memo_at_limit_fails() {
        let memo = "refu";
        let mt = create_single_event_mt(defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT, Some(memo));
        assert!(matches!(mt.check_refund().unwrap_err(), ErrorLogTooLong));
    }

    #[test]
    fn triple_event_no_memo_at_limit_minus_overhead_passes() {
        let mt = create_triple_event_mt(
            defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT - 3 * REFUND_EXTRA_BYTES,
            [None; 3],
        );
        assert!(mt.check_refund().is_ok());
    }

    #[test]
    fn triple_event_short_memo_at_limit_fails() {
        let mt =
            create_triple_event_mt(defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT, [Some("refu"); 3]);
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
            defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT,
            [None, Some("refund"), Some(&long_memo)],
        );
        assert!(mt.check_refund().is_ok());
    }
}
