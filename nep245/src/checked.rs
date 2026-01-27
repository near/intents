use defuse_near_utils::REFUND_MEMO;
use near_sdk::FunctionError;

pub use defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT;

#[derive(Debug, Clone, PartialEq, Eq, FunctionError, thiserror::Error)]
#[error("event log is too long: {log_length} bytes exceeds limit of {limit} bytes")]
pub struct ErrorLogTooLong {
    pub log_length: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, FunctionError, thiserror::Error)]
#[error("refund event log would be too long: {log_length} bytes exceeds limit of {limit} bytes")]
pub struct ErrorRefundLogTooLong {
    pub log_length: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, FunctionError, thiserror::Error)]
pub enum CheckRefundError {
    #[error(transparent)]
    LogTooLong(#[from] ErrorLogTooLong),
    #[error(transparent)]
    RefundLogTooLong(#[from] ErrorRefundLogTooLong),
}

const REFUND_STR_LEN: usize = REFUND_MEMO.len();
pub const REFUND_EXTRA_BYTES: usize = r#","memo":""#.len() + REFUND_STR_LEN;

#[derive(Default, Clone, Copy)]
#[must_use]
pub struct RefundLogDelta {
    pub overhead: usize,
    pub savings: usize,
}

impl RefundLogDelta {
    pub const fn saturating_add(self, other: Self) -> Self {
        Self {
            overhead: self.overhead.saturating_add(other.overhead),
            savings: self.savings.saturating_add(other.savings),
        }
        .normalize()
    }

    const fn normalize(self) -> Self {
        let common = if self.overhead < self.savings {
            self.overhead
        } else {
            self.savings
        };
        Self {
            overhead: self.overhead - common,
            savings: self.savings - common,
        }
    }
}

pub const fn refund_log_delta(memo: Option<&str>) -> RefundLogDelta {
    let Some(m) = memo else {
        return RefundLogDelta {
            overhead: REFUND_EXTRA_BYTES,
            savings: 0,
        };
    };
    RefundLogDelta {
        overhead: REFUND_STR_LEN.saturating_sub(m.len()),
        savings: m.len().saturating_sub(REFUND_STR_LEN),
    }
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
