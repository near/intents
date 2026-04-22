use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use thiserror::Error;
use tower::Service;

use crate::types::{AccountId, ProjectEnv};

#[derive(Debug, Error)]
pub enum EnvFetchError {
    #[error("NEAR RPC call failed: {0}")]
    Rpc(String),
    #[error("decryption failed: {0}")]
    Decryption(String),
}

/// Fetches and decrypts the project's environment variables / secrets.
///
/// TODO: query NEAR RPC for the encrypted env blob keyed by `project_id`,
/// then decrypt using the TEE-held key. Currently returns an empty placeholder.
#[derive(Clone)]
pub struct EnvFetchService;

impl Service<AccountId> for EnvFetchService {
    type Response = ProjectEnv;
    type Error = EnvFetchError;
    type Future = BoxFuture<'static, Result<ProjectEnv, EnvFetchError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), EnvFetchError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _project_id: AccountId) -> Self::Future {
        Box::pin(async move { Ok(ProjectEnv) })
    }
}
