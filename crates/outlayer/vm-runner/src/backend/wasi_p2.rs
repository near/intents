use anyhow::Result;
use defuse_outlayer_sys::host::Host;
use wasmtime::Store;
use wasmtime::component::{Component, Linker};
use wasmtime_wasi::p2::bindings::Command;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::backend::WasiBackend;
use crate::host::HostCtx;

pub struct WasiP2State {
    wasi_ctx: WasiCtx,
    resource_table: wasmtime_wasi::ResourceTable,
}

impl WasiP2State {
    pub fn new(wasi_ctx: WasiCtx) -> Self {
        Self {
            wasi_ctx,
            resource_table: wasmtime_wasi::ResourceTable::new(),
        }
    }
}

impl<T: Host> WasiView for HostCtx<WasiP2State, T> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_state.wasi_ctx,
            table: &mut self.wasi_state.resource_table,
        }
    }
}

pub struct WasiP2Backend;

impl WasiBackend for WasiP2Backend {
    type State = WasiP2State;

    fn setup_linker<H: Host + 'static>(linker: &mut Linker<HostCtx<WasiP2State, H>>) -> Result<()> {
        wasmtime_wasi::p2::add_to_linker_async(linker)
    }

    fn build_state(
        stdin: MemoryInputPipe,
        stdout: MemoryOutputPipe,
        stderr: MemoryOutputPipe,
    ) -> Result<WasiP2State> {
        Ok(WasiP2State::new(
            WasiCtx::builder()
                .stdin(stdin)
                .stdout(stdout)
                .stderr(stderr)
                .build(),
        ))
    }

    async fn call_run<H: Host + 'static>(
        store: &mut Store<HostCtx<WasiP2State, H>>,
        component: &Component,
        linker: &Linker<HostCtx<WasiP2State, H>>,
    ) -> anyhow::Result<()> {
        let command = Command::instantiate_async(&mut *store, component, linker).await?;

        command
            .wasi_cli_run()
            .call_run(&mut *store)
            .await
            .map(|_| ())
    }
}
