pub use defuse_outlayer_primitives as primitives;
use defuse_outlayer_primitives::AppId;

use crate::crypto::Signer;

mod crypto;

// TODO: rename
pub mod bindings {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "imports",
        imports: {
            // default: async | trappable,
        },
        ownership: Borrowing {
            duplicate_if_necessary: true
        },
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context<'a> {
    pub app_id: AppId<'a>,
}

pub struct Host {
    ctx: Context<'static>,
    signer: Box<dyn Signer>,
}

impl Host {
    pub fn new(ctx: Context<'static>, signer: impl Signer + 'static) -> Self {
        Self {
            ctx,
            signer: Box::new(signer),
        }
    }

    pub fn with_app_id(&mut self, app_id: impl Into<AppId<'static>>) -> &mut Self {
        self.app_id = app_id.into();
        self
    }
}
