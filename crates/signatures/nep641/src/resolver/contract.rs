use near_account_id::AccountId;
use near_kit::{BlockReference, RpcError};
#[cfg(feature = "tracing")]
use tracing::instrument;

use crate::{
    client::WResolveAuthArgs,
    resolver::{ResolveErrorKind, Resolved, RpcResolver},
};

impl RpcResolver {
    /// Try to resolve a single authorization via `w_resolve_auth()` view-method
    #[cfg_attr(feature = "tracing", instrument(level = "DEBUG", skip_all))]
    pub(super) async fn resolve_contract(
        &self,
        account_id: &AccountId,
        path: &[AccountId],
        authorization: &str,
        block: BlockReference,
    ) -> Result<Resolved, ResolveErrorKind> {
        let res = self
            .client
            .view_function(
                account_id,
                "w_resolve_auth",
                &serde_json::to_vec(&WResolveAuthArgs {
                    path,
                    authorization,
                })
                .expect("JSON: serialization failed"),
                block,
                // TODO: "pre-init" if we have StateInit for this AccountId
                // self.state_inits.get(&account_id),
            )
            .await
            .map_err::<ResolveErrorKind, _>(|err| match err {
                RpcError::AccountNotFound(_) | RpcError::ContractNotDeployed(_) => {
                    ContractError::NoResolve.into()
                }
                RpcError::ContractPanic { message }
                | RpcError::ContractExecution { message, .. } => {
                    ContractError::Panic(message).into()
                }
                RpcError::FunctionCall { panic, .. } => {
                    ContractError::Panic(panic.unwrap_or_else(|| "contract panic".to_string()))
                        .into()
                }
                _ => err.into(),
            })?;

        Ok(Resolved {
            res: res.json()?,
            block_hash: res.block_hash,
            block_height: res.block_height,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContractError {
    #[error("account does not exist, has not been initialized yet or has no contract deployed")]
    NoResolve,

    #[error("{0}")] // propagate errors directly from the contract
    Panic(String),
}
