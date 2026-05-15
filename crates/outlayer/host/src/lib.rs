pub mod bindings;
pub mod crypto;

use std::sync::Arc;

pub use defuse_outlayer_primitives as primitives;
use defuse_outlayer_primitives::AppId;
pub use defuse_outlayer_signer::InMemorySigner;
use wasmtime::component::Linker;

use crate::crypto::Signer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context<'a> {
    pub app_id: AppId<'a>,
}

impl<'a> Context<'a> {
    pub fn as_ref<'b: 'a>(&'b self) -> Context<'b> {
        Context {
            app_id: self.app_id.as_ref(),
        }
    }
}

pub struct Host<'a> {
    ctx: Context<'a>,
    signer: Arc<dyn Signer>,
}

impl<'a> Host<'a> {
    #[inline]
    pub const fn new(ctx: Context<'a>, signer: Arc<dyn Signer>) -> Self {
        Self { ctx, signer }
    }
}

pub trait HostView: Send {
    fn ctx(&mut self) -> Host<'_>;
}

impl HostView for Host<'_> {
    fn ctx(&mut self) -> Host<'_> {
        Host {
            ctx: self.ctx.as_ref(),
            signer: self.signer.clone(),
        }
    }
}

pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: HostView,
{
    crypto::add_to_linker::<T>(linker)
}
