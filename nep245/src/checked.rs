use defuse_near_utils::REFUND_MEMO;
use near_sdk::FunctionError;

pub use defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT;

#[derive(Debug, Clone, PartialEq, Eq, FunctionError, thiserror::Error)]
#[error("refund event log would be too long")]
pub struct ErrorLogTooLong {}

const REFUND_STR_LEN: usize = REFUND_MEMO.len();
#[allow(clippy::redundant_pub_crate)]
pub(crate) const REFUND_EXTRA_BYTES: usize = r#","memo":""#.len() + REFUND_STR_LEN;

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

pub const fn refund_log_delta(memo: Option<&str>) -> RefundLogDelta {
    let Some(m) = memo else {
        return RefundLogDelta {
            overhead: REFUND_EXTRA_BYTES,
            savings: 0,
        };
    };
    RefundLogDelta::new(
        REFUND_STR_LEN.saturating_sub(m.len()),
        m.len().saturating_sub(REFUND_STR_LEN),
    )
}

/// A validated event log that has been checked for refund overhead.
/// Use [`RefundCheckedMtEvent::emit`] to emit the event.
#[derive(Debug)]
#[must_use = "call `.emit()` to emit the event"]
pub struct RefundCheckedMtEvent(pub String);

impl RefundCheckedMtEvent {
    pub fn emit(self) {
        near_sdk::env::log_str(&self.0);
    }
}
