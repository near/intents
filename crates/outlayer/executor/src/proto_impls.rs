use defuse_outlayer_proto as proto;

use crate::Outcome;

impl From<Outcome> for proto::ExecuteResponse {
    fn from(v: Outcome) -> Self {
        Self {
            output: v.output.into(),
            logs: v.logs.into(),
            execution: Some(v.execution.into()),
        }
    }
}
