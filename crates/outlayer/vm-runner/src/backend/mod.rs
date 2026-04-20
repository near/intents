pub mod wasi_p2;

use anyhow::Result;
use defuse_outlayer_sys::host::Host;
use wasmtime::Store;
use wasmtime::component::{Component, Linker};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use crate::host::HostCtx;

#[allow(async_fn_in_trait)]
pub trait WasiBackend: Send + Sync {
    type State: Send + 'static;

    fn setup_linker<H: Host>(linker: &mut Linker<HostCtx<Self::State, H>>) -> Result<()>;

    fn build_state(
        stdin: MemoryInputPipe,
        stdout: MemoryOutputPipe,
        stderr: MemoryOutputPipe,
    ) -> Self::State;

    async fn call_run<H: Host>(
        store: &mut Store<HostCtx<Self::State, H>>,
        component: &Component,
        linker: &Linker<HostCtx<Self::State, H>>,
    ) -> anyhow::Result<()>;
}
