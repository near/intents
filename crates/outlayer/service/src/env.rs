use std::future::{Ready, ready};
use std::task::{Context, Poll};

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

#[derive(Clone)]
pub struct EnvFetchService;

impl Service<AccountId> for EnvFetchService {
    type Response = ProjectEnv;
    type Error = EnvFetchError;
    type Future = Ready<Result<ProjectEnv, EnvFetchError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), EnvFetchError>> {
        Poll::Ready(Ok(()))
    }

    #[tracing::instrument(level = "debug", name = "env.fetch", skip_all)]
    fn call(&mut self, _project_id: AccountId) -> Self::Future {
        ready(Ok(ProjectEnv))
    }
}
