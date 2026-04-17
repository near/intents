pub struct ExecutionOutcome {
    pub output: Vec<u8>,
    pub stderr: String,
    pub fuel_consumed: u64,
}
