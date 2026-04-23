use crate::error::ExecutionError;

/// Outcome of executing a component in the VM runtime
#[derive(Debug)]
pub struct ExecutionOutcome {
    pub fuel_consumed: u64,
    pub guest_error: Option<ExecutionError>,
}
