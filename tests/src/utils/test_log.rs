use near_workspaces::result::ExecutionResult;

#[allow(dead_code)]
#[derive(Debug)]
pub struct TestLog {
    logs: Vec<String>,
    receipt_failure_errors: Vec<String>,
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
        }
    }
}

impl TestLog {
    pub fn logs(&self) -> &[String] {
        &self.logs
    }
}
