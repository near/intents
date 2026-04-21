/// Outcome of executing a component in the VM runtime
#[derive(Debug)]
pub struct ExecutionOutcome {
    pub output: Vec<u8>,
    pub stderr: Vec<u8>,
    pub fuel_consumed: u64,
}
