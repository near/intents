use defuse_outlayer_sys::host::Host;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

pub struct WasiP2State {
    pub wasi_ctx: WasiCtx,
    pub resource_table: wasmtime_wasi::ResourceTable,
}

impl WasiP2State {
    pub fn new(wasi_ctx: WasiCtx) -> Self {
        Self {
            wasi_ctx,
            resource_table: wasmtime_wasi::ResourceTable::new(),
        }
    }
}

pub struct HostCtx<T: Host> {
    pub wasip2_state: WasiP2State,
    pub host_state: T,
}

impl<T: Host> HostCtx<T> {
    pub fn new(wasip2_state: WasiP2State, host_state: T) -> Self {
        Self {
            wasip2_state,
            host_state,
        }
    }
}

impl<T: Host> WasiView for HostCtx<T> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasip2_state.wasi_ctx,
            table: &mut self.wasip2_state.resource_table,
        }
    }
}
