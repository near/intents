/// Maximum length of a single log entry in NEAR runtime.
/// See: <https://github.com/near/nearcore/blob/v2.5.0/runtime/near-vm-runner/src/logic/logic.rs#L42>
pub const TOTAL_LOG_LENGTH_LIMIT: usize = 16384;

/// Memo used for refund events.
pub const REFUND_MEMO: &str = "refund";

pub trait NearSdkLog {
    fn to_near_sdk_log(&self) -> String;
}
