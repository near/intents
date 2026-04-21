use defuse_outlayer_host_functions::HostFunctions;
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

    pub const fn host_state_mut(&mut self) -> &mut T {
        &mut self.host_state
    }

    const fn wasi_state_mut(&mut self) -> &mut WasiP2State {
        &mut self.wasi_state
    }

    pub(crate) const fn limits_mut(&mut self) -> &mut StoreLimits {
        &mut self.limits
    }
}

impl<T: HostFunctions> WasiView for HostCtx<T> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        let state = self.wasi_state_mut();
        WasiCtxView {
            ctx: &mut state.wasi_ctx,
            table: &mut state.resource_table,
        }
    }
}

struct WasiP2State {
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
