use near_api::types::transaction::result::{
    ExecutionFinalResult, ExecutionOutcome, ValueOrReceiptId,
};
use std::fmt::Debug;

pub struct TxOutcome<'a>(&'a ExecutionFinalResult);

// TODO: add tracing
impl Debug for TxOutcome<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} -> {}: ",
            self.0.transaction().signer_id(),
            self.0.transaction().receiver_id()
        )?;
        let outcomes: Vec<_> = self
            .0
            .outcomes()
            .into_iter()
            .map(TestExecutionOutcome)
            .collect();
        if !outcomes.is_empty() {
            f.debug_list().entries(outcomes).finish()?;
        }
        Ok(())
    }
}

impl<'a> From<&'a ExecutionFinalResult> for TxOutcome<'a> {
    fn from(value: &'a ExecutionFinalResult) -> Self {
        TxOutcome(value)
    }
}

pub struct TestExecutionOutcome<'a>(&'a ExecutionOutcome);

impl Debug for TestExecutionOutcome<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ({}) ", self.0.executor_id, self.0.gas_burnt)?;
        if !self.0.logs.is_empty() {
            f.debug_list().entries(&self.0.logs).finish()?;
        }
        match self.0.clone().into_result() {
            Ok(v) => {
                if let ValueOrReceiptId::Value(value) = v {
                    let bytes = value.raw_bytes().unwrap();
                    if !bytes.is_empty() {
                        if bytes.len() <= 32 {
                            write!(f, ", OK: {bytes:?}")?;
                        } else {
                            write!(
                                f,
                                ", OK: {:?}..{:?}",
                                &bytes[..16],
                                &bytes[bytes.len() - 16..]
                            )?;
                        }
                    }
                }
                Ok(())
            }
            Err(err) => write!(f, ", FAIL: {err:#?}"),
        }
    }
}
