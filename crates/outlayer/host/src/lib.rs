pub mod bindings;
pub mod crypto;

use std::sync::Arc;

pub use defuse_outlayer_crypto::signer::InMemorySigner;
pub use defuse_outlayer_primitives as primitives;
use defuse_outlayer_primitives::AppId;
use wasmtime::component::{HasData, Linker};

use crate::bindings::Imports;

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
    signer: Arc<InMemorySigner>,
}

impl<'a> Host<'a> {
    pub fn new(ctx: Context<'a>, signer: impl Into<Arc<InMemorySigner>>) -> Self {
        Self {
            ctx,
            signer: signer.into(),
        }
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

struct HasHost;

impl HasData for HasHost {
    type Data<'a> = Host<'a>;
}

pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: HostView,
{
    Imports::add_to_linker::<T, HasHost>(linker, T::ctx)
}
