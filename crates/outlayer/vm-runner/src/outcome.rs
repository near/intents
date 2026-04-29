use crate::error::ExecutionError;

/// Outcome of executing a component in the VM runtime
#[derive(Debug)]
pub struct ExecutionOutcome {
    pub fuel_consumed: u64,
    pub guest_error: Option<ExecutionError>,
}

impl ExecutionOutcome {
    pub fn into_result(self) -> Result<(), ExecutionError> {
        if let Some(err) = self.guest_error {
            return Err(err);
        }
        Ok(())
    }
}
