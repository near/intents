//! Workaround for a bug in `near-api` where `DeterministicStateInit` transactions
//! report `TransportError` even when the transaction actually succeeded.
//!
//! This module should be deleted when the upstream fix is merged:
//! <https://github.com/PolyProgrammist/near-openapi-client/pull/32>

use near_api::errors::{ExecuteTransactionError, RetryError, SendRequestError};
use near_openapi_client::Error as OpenApiError;
use near_sdk::serde_json;

use crate::TxError;

/// Extension trait for `Result<T, ExecuteTransactionError>` that handles
/// a bug in near-api where `DeterministicStateInit` transactions report
/// `TransportError` even when the transaction succeeded.
pub trait UnwrapGlobalContractDeployment<T> {
    /// Returns `true` if the result is `Ok`, or if it's a specific `Err` variant
    /// where the inner JSON payload indicates success via `"status":{"SuccessValue":...}`.
    fn unwrap_global_contract_deployment(self) -> bool;
}

impl<T> UnwrapGlobalContractDeployment<T> for Result<T, ExecuteTransactionError> {
    fn unwrap_global_contract_deployment(self) -> bool {
        match self {
            Ok(_) => true,
            Err(ExecuteTransactionError::TransactionError(retry_error)) => {
                is_success_in_transport_error(&retry_error)
            }
            Err(_) => false,
        }
    }
}

impl<T> UnwrapGlobalContractDeployment<T> for Result<T, TxError> {
    fn unwrap_global_contract_deployment(self) -> bool {
        // TxError is anyhow::Error, so we try to downcast
        match self {
            Ok(_) => true,
            Err(err) => {
                if let Some(exec_err) = err.downcast_ref::<ExecuteTransactionError>() {
                    if let ExecuteTransactionError::TransactionError(retry_error) = exec_err {
                        return is_success_in_transport_error(retry_error);
                    }
                }
                false
            }
        }
    }
}

fn is_success_in_transport_error<RpcError: std::fmt::Debug + Send + Sync>(
    retry_error: &RetryError<SendRequestError<RpcError>>,
) -> bool {
    let (RetryError::Critical(SendRequestError::TransportError(
        OpenApiError::InvalidResponsePayload(bytes, _),
    ))
    | RetryError::RetriesExhausted(SendRequestError::TransportError(
        OpenApiError::InvalidResponsePayload(bytes, _),
    ))) = retry_error
    else {
        return false;
    };

    // Parse the JSON and check for SuccessValue in status
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };

    // Check result.status.SuccessValue exists
    json.get("result")
        .and_then(|r| r.get("status"))
        .and_then(|s| s.get("SuccessValue"))
        .is_some()
}
