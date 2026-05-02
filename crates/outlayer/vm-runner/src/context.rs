use defuse_outlayer_host::HostFunctions;
use wasmtime::StoreLimits;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};


/// The host context passed to the component, containing both
/// the WASI state and the custom host state
///
/// Used as the context for the linker when instantiating the component, and
/// passed to host functions when called by the component
pub struct HostCtx<T: HostFunctions> {
    wasi_state: WasiP2State,
    host_state: T,
    limits: StoreLimits,
}

impl<T: HostFunctions> HostCtx<T> {
    pub fn new(wasi_state: WasiCtx, host_state: T, limits: StoreLimits) -> Self {
        Self {
            wasi_state: WasiP2State::new(wasi_state),
            host_state,
            limits,
        }
    }

    pub(crate) const fn limits_mut(&mut self) -> &mut StoreLimits {
        &mut self.limits
    }

    pub(crate) const fn host_state_mut(&mut self) -> &mut T {
        &mut self.host_state
    }
}

impl<T: HostFunctions> WasiView for HostCtx<T> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        self.wasi_state.ctx()
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
