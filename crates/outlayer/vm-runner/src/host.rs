use defuse_outlayer_sys::host::Host;
use wasmtime::StoreLimits;

pub struct HostCtx<W, T: Host> {
    pub wasi_state: W,
    pub host_state: T,
    pub limits: StoreLimits,
}

impl<W, T: Host> HostCtx<W, T> {
    pub const fn new(wasi_state: W, host_state: T, limits: StoreLimits) -> Self {
        Self {
            wasi_state,
            host_state,
            limits,
        }
    }
}
