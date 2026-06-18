use near_kit::{ExecutionOutcomeWithId, ExecutionStatus, FinalExecutionOutcome};

#[derive(Debug)]
pub struct SuccessfulExecutionOutcome {
    pub transaction_outcome: ExecutionOutcomeWithId,
    pub receipts_outcome: Vec<ExecutionOutcomeWithId>,
}

impl SuccessfulExecutionOutcome {
    pub fn logs(&self) -> Vec<String> {
        self.transaction_outcome
            .outcome
            .logs
            .iter()
            .chain(
                self.receipts_outcome
                    .iter()
                    .flat_map(|o| o.outcome.logs.iter()),
            )
            .cloned()
            .collect()
    }

    // TODO: use it where possible
    pub fn is_success(&self) -> bool {
        matches!(
            self.transaction_outcome.outcome.status,
            ExecutionStatus::SuccessValue(_)
        ) && self
            .receipts_outcome
            .iter()
            .all(|o| matches!(o.outcome.status, ExecutionStatus::SuccessValue(_)))
    }
}

impl TryFrom<FinalExecutionOutcome> for SuccessfulExecutionOutcome {
    type Error = anyhow::Error;
    fn try_from(outcome: FinalExecutionOutcome) -> Result<Self, Self::Error> {
        outcome.result()?;
        Ok(Self {
            transaction_outcome: outcome.transaction_outcome,
            receipts_outcome: outcome.receipts_outcome,
        })
    }
}
