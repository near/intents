use near_sdk::Gas;
use near_workspaces::result::ExecutionResult;

#[allow(dead_code)]
#[derive(Debug)]
pub struct TestLog {
    logs: Vec<String>,
    receipt_failure_errors: Vec<String>,
    gas_burnt_in_tx: Gas,
    logs_and_gas_burnt_in_receipts: Vec<(Vec<String>, Gas)>,
}

impl From<ExecutionResult<near_workspaces::result::Value>> for TestLog {
    fn from(outcome: ExecutionResult<near_workspaces::result::Value>) -> Self {
        Self {
            logs: outcome.logs().into_iter().map(str::to_string).collect(),
            receipt_failure_errors: outcome
                .receipt_outcomes()
                .iter()
                .map(|s| {
                    if let Err(e) = (*s).clone().into_result() {
                        match e.into_inner() {
                            Ok(o) => format!("OK: {o}"),
                            Err(e) => format!("Err: {e}"),
                        }
                    } else {
                        String::new()
                    }
                })
                .collect::<Vec<_>>(),
            gas_burnt_in_tx: outcome.total_gas_burnt,
            logs_and_gas_burnt_in_receipts: outcome
                .receipt_outcomes()
                .iter()
                .map(|v| (v.logs.clone(), v.gas_burnt))
                .collect(),
        }
    }
}

impl TestLog {
    pub fn logs(&self) -> &[String] {
        &self.logs
    }

    pub const fn total_gas_burnt(&self) -> &Gas {
        &self.gas_burnt_in_tx
    }

    pub const fn logs_and_gas_burnt_in_receipts(&self) -> &Vec<(Vec<String>, Gas)> {
        &self.logs_and_gas_burnt_in_receipts
    }
}
