use defuse_outlayer_host_functions::HostFunctions;
use wasmtime::StoreLimits;

pub struct HostCtx<W, T: HostFunctions> {
    pub wasi_state: W,
    pub host_state: T,
    pub limits: StoreLimits,
}

impl<W, T: HostFunctions> HostCtx<W, T> {
    pub const fn new(wasi_state: W, host_state: T, limits: StoreLimits) -> Self {
        Self {
            wasi_state,
            host_state,
            limits,
        }
    }
}
