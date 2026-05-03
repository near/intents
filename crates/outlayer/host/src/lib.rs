pub mod bindings;
pub mod crypto;

use std::sync::Arc;

pub use defuse_outlayer_crypto::signer::InMemorySigner;
pub use defuse_outlayer_primitives as primitives;
use defuse_outlayer_primitives::AppId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppContext {
    pub app_id: AppId<'static>,
}

pub struct State {
    ctx: AppContext,
    signer: Arc<InMemorySigner>,
}

impl State {
    pub fn new(ctx: AppContext, signer: impl Into<Arc<InMemorySigner>>) -> Self {
        Self {
            ctx,
            signer: signer.into(),
        }
    }
}
