use near_api::types::transaction::{
    actions::Action,
    result::{ExecutionFinalResult, ExecutionResult},
};
use near_sdk::{AccountId, Gas};

/// Gas information for an execution outcome
#[derive(Debug, Clone)]
pub struct GasInfo {
    pub executor_id: AccountId,
    pub gas_burnt: Gas,
    pub method_name: Option<String>,
}

/// Extension trait for querying gas usage from execution results
pub trait ExecutionResultExt {
    /// Get gas info for all outcomes matching the given executor
    fn gas_by_executor(&self, executor: &AccountId) -> Vec<GasInfo>;

    /// Get gas info for transaction outcome matching receiver + method name.
    /// Returns None if no match (only checks the transaction, not receipts
    /// since method names are not available for receipt outcomes).
    fn gas_by_method(&self, receiver: &AccountId, method: &str) -> Option<GasInfo>;
}

impl ExecutionResultExt for ExecutionFinalResult {
    fn gas_by_executor(&self, executor: &AccountId) -> Vec<GasInfo> {
        self.outcomes()
            .into_iter()
            .filter(|o| &o.executor_id == executor)
            .map(|o| GasInfo {
                executor_id: o.executor_id.clone(),
                gas_burnt: Gas::from_gas(o.gas_burnt.as_gas()),
                method_name: None,
            })
            .collect()
    }

    fn gas_by_method(&self, receiver: &AccountId, method: &str) -> Option<GasInfo> {
        let tx = self.transaction();
        if tx.receiver_id() != receiver {
            return None;
        }

        let has_method = tx.actions().iter().any(|action| {
            if let Action::FunctionCall(fn_call) = action {
                fn_call.method_name == method
            } else {
                false
            }
        });

        if has_method {
            let outcome = self.outcome();
            Some(GasInfo {
                executor_id: outcome.executor_id.clone(),
                gas_burnt: Gas::from_gas(outcome.gas_burnt.as_gas()),
                method_name: Some(method.to_string()),
            })
        } else {
            None
        }
    }
}

impl<T> ExecutionResultExt for ExecutionResult<T> {
    fn gas_by_executor(&self, executor: &AccountId) -> Vec<GasInfo> {
        self.outcomes()
            .into_iter()
            .filter(|o| &o.executor_id == executor)
            .map(|o| GasInfo {
                executor_id: o.executor_id.clone(),
                gas_burnt: Gas::from_gas(o.gas_burnt.as_gas()),
                method_name: None,
            })
            .collect()
    }

    fn gas_by_method(&self, receiver: &AccountId, method: &str) -> Option<GasInfo> {
        let tx = self.transaction();
        if tx.receiver_id() != receiver {
            return None;
        }

        let has_method = tx.actions().iter().any(|action| {
            if let Action::FunctionCall(fn_call) = action {
                fn_call.method_name == method
            } else {
                false
            }
        });

        if has_method {
            let outcome = self.outcome();
            Some(GasInfo {
                executor_id: outcome.executor_id.clone(),
                gas_burnt: Gas::from_gas(outcome.gas_burnt.as_gas()),
                method_name: Some(method.to_string()),
            })
        } else {
            None
        }
    }
}
