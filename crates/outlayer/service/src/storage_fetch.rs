use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use thiserror::Error;
use tower::Service;

use crate::types::{AccountId, ProjectStorage};

#[derive(Debug, Error)]
pub enum StorageFetchError {
    #[error("NEAR RPC call failed: {0}")]
    Rpc(String),
    #[error("storage not found for project: {0}")]
    NotFound(String),
}

/// Fetches the project's on-chain storage state (key → value snapshot).
///
/// TODO: query NEAR RPC for the storage trie rooted at `project_id`,
/// returning a snapshot consistent with `tx_hash`. Currently returns an
/// empty placeholder.
#[derive(Clone)]
pub struct StorageFetchService;

impl Service<AccountId> for StorageFetchService {
    type Response = ProjectStorage;
    type Error = StorageFetchError;
    type Future = BoxFuture<'static, Result<ProjectStorage, StorageFetchError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), StorageFetchError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _project_id: AccountId) -> Self::Future {
        Box::pin(async move { Ok(ProjectStorage) })
    }
}
