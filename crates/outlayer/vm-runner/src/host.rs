use defuse_outlayer_sys::host::Host;

pub struct HostCtx<W, T: Host> {
    pub wasi_state: W,
    pub host_state: T,
}

impl<W, T: Host> HostCtx<W, T> {
    pub fn new(wasi_state: W, host_state: T) -> Self {
        Self {
            wasi_state,
            host_state,
        }
    }
}
