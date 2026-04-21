use anyhow::Result;
use defuse_outlayer_host_functions::HostFunctions;
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
    fn new(wasi_ctx: WasiCtx) -> Self {
        Self {
            wasi_ctx,
            resource_table: wasmtime_wasi::ResourceTable::new(),
        }
    }
}

impl<T: HostFunctions> WasiView for HostCtx<WasiP2State, T> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        let state = self.wasi_state_mut();
        WasiCtxView {
            ctx: &mut state.wasi_ctx,
            table: &mut state.resource_table,
        }
    }
}

pub struct WasiP2Backend;

impl WasiBackend for WasiP2Backend {
    type State = WasiP2State;

    fn setup_linker<H: HostFunctions>(linker: &mut Linker<HostCtx<WasiP2State, H>>) -> Result<()> {
        wasmtime_wasi::p2::add_to_linker_async(linker)
    }

    fn build_state(
        stdin: MemoryInputPipe,
        stdout: MemoryOutputPipe,
        stderr: MemoryOutputPipe,
    ) -> WasiP2State {
        WasiP2State::new(
            WasiCtx::builder()
                .stdin(stdin)
                .stdout(stdout)
                .stderr(stderr)
                .build(),
        )
    }

    async fn run<H: HostFunctions>(
        store: &mut Store<HostCtx<WasiP2State, H>>,
        component: &Component,
        linker: &Linker<HostCtx<WasiP2State, H>>,
    ) -> anyhow::Result<()> {
        let command = Command::instantiate_async(&mut *store, component, linker).await?;

        command
            .wasi_cli_run()
            .call_run(&mut *store)
            .await?
            .map_err(|()| anyhow::anyhow!("component run() returned Err(())"))
    }
}
