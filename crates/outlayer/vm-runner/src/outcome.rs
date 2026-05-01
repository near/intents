use crate::error::ExecutionError;

/// Details about the execution of a component
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
#[non_exhaustive]
pub struct ExecutionDetails {
    pub fuel_consumed: u64,
}

/// Outcome of executing a component in the VM runtime
#[derive(Debug, Default)]
pub struct ExecutionOutcome {
    pub details: ExecutionDetails,
    pub error: Option<ExecutionError>,
}

impl ExecutionOutcome {
    pub fn into_result(self) -> Result<(), ExecutionError> {
        self.error.map_or(Ok(()), Err)
    }
}
