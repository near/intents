use bytes::Bytes;
use defuse_outlayer_vm_runner::{ExecutionError, ExecutionOutcome};

pub struct Response {
    pub output: Bytes,
    pub logs: Bytes,
    pub outcome: ExecutionOutcome,
}

impl Response {
    pub fn into_result(self) -> Result<Bytes, ExecutionError> {
        self.outcome.into_result().map(|()| self.output)
    }
}
