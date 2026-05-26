use defuse_outlayer_proto as proto;

use crate::{ExecutionDetails, ExecutionOutcome};

impl TryFrom<proto::ExecutionDetails> for ExecutionDetails {
    type Error = String;

    fn try_from(p: proto::ExecutionDetails) -> Result<Self, Self::Error> {
        Ok(Self {
            fuel_consumed: p.fuel_consumed,
        })
    }
}

impl TryFrom<proto::ExecutionOutcome> for ExecutionOutcome {
    type Error = String;

    fn try_from(p: proto::ExecutionOutcome) -> Result<Self, Self::Error> {
        let details = p
            .details
            .ok_or_else(|| "missing ExecutionOutcome.details".to_owned())?
            .try_into()?;
        Ok(Self {
            details,
            error: p.error,
        })
    }
}

impl From<ExecutionDetails> for proto::ExecutionDetails {
    fn from(v: ExecutionDetails) -> Self {
        Self {
            fuel_consumed: v.fuel_consumed,
        }
    }
}

impl From<ExecutionOutcome> for proto::ExecutionOutcome {
    fn from(v: ExecutionOutcome) -> Self {
        Self {
            details: Some(v.details.into()),
            error: v.error,
        }
    }
}
