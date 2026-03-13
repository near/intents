use crate::MtEvent;
use defuse_near_utils::REFUND_MEMO;
use near_sdk::FunctionError;

#[derive(Debug, Clone, PartialEq, Eq, FunctionError, thiserror::Error)]
#[error("refund event log would be too long")]
pub struct ErrorLogTooLong;

const REFUND_STR_LEN: usize = REFUND_MEMO.len();
pub const REFUND_EXTRA_BYTES: usize = r#","memo":""#.len() + REFUND_STR_LEN;

#[derive(Default, Clone, Copy)]
#[must_use]
pub struct RefundLogDelta {
    overhead: usize,
    savings: usize,
}

impl RefundLogDelta {
    pub const fn new(overhead: usize, savings: usize) -> Self {
        Self {
            overhead: overhead.saturating_sub(savings),
            savings: savings.saturating_sub(overhead),
        }
    }

    pub const fn overhead(&self) -> usize {
        self.overhead
    }

    pub const fn savings(&self) -> usize {
        self.savings
    }

    pub const fn saturating_add(self, other: Self) -> Self {
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
    pub(crate) fn compute_refund_delta(&self) -> RefundLogDelta {
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

#[cfg(test)]
mod test {
    use crate::checked::REFUND_EXTRA_BYTES;

    use super::refund_log_delta;

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
}
