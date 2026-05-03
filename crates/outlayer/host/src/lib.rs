pub mod bindings;
pub mod crypto;

use std::sync::Arc;

pub use defuse_outlayer_crypto::signer::InMemorySigner;
pub use defuse_outlayer_primitives as primitives;
use defuse_outlayer_primitives::AppId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub app_id: AppId<'static>,
}

pub struct State {
    ctx: Context,
    signer: Arc<InMemorySigner>,
}

impl State {
    pub fn new(ctx: Context, signer: impl Into<Arc<InMemorySigner>>) -> Self {
        Self {
            ctx,
            signer: signer.into(),
        }
    }
}
