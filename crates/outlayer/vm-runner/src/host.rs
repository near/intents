use defuse_outlayer_host_functions::HostFunctions;
use wasmtime::StoreLimits;

pub struct HostCtx<W, T: HostFunctions> {
    wasi_state: W,
    host_state: T,
    limits: StoreLimits,
}

impl<W, T: HostFunctions> HostCtx<W, T> {
    pub const fn new(wasi_state: W, host_state: T, limits: StoreLimits) -> Self {
        Self {
            wasi_state,
            host_state,
            limits,
        }
    }

    pub const fn host_state_mut(&mut self) -> &mut T {
        &mut self.host_state
    }

    pub const fn wasi_state_mut(&mut self) -> &mut W {
        &mut self.wasi_state
    }

    pub(crate) const fn limits_mut(&mut self) -> &mut StoreLimits {
        &mut self.limits
    }
}
