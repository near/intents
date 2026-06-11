use near_kit::{ExecutionOutcomeWithId, FinalExecutionOutcome};

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
