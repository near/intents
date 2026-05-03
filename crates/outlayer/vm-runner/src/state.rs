use defuse_outlayer_host::State as HostState;
use wasmtime::StoreLimits;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

/// The host context passed to the component, containing both
/// the WASI state and the custom host state
///
/// Used as the context for the linker when instantiating the component, and
/// passed to host functions when called by the component
pub struct State {
    wasi: WasiP2State,
    host: HostState,
    limits: StoreLimits,
}

impl State {
    pub fn new(wasi: WasiCtx, host: HostState, limits: StoreLimits) -> Self {
        Self {
            wasi: WasiP2State::new(wasi),
            host,
            limits,
        }
    }

    pub(crate) const fn limits_mut(&mut self) -> &mut StoreLimits {
        &mut self.limits
    }

    pub(crate) const fn host_state_mut(&mut self) -> &mut HostState {
        &mut self.host
    }
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        self.wasi.ctx()
    }
}

struct WasiP2State {
    ctx: WasiCtx,
    table: wasmtime_wasi::ResourceTable,
}

impl WasiP2State {
    fn new(ctx: WasiCtx) -> Self {
        Self {
            ctx,
            table: wasmtime_wasi::ResourceTable::new(),
        }
    }
}

impl WasiView for WasiP2State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}
