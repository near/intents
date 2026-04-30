use defuse_outlayer_host::HostFunctions;
use wasmtime::StoreLimits;
use wasmtime::component::HasData;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::bindings;

/// The host context passed to the component, containing both
/// the WASI state and the custom host state
///
/// Used as the context for the linker when instantiating the component, and
/// passed to host functions when called by the component
pub struct HostCtx<T> {
    wasi_state: WasiP2State,
    host_state: T,
    limits: StoreLimits,
}

impl<T> HostCtx<T> {
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

    pub const fn host_state_mut(&mut self) -> &mut T {
        &mut self.host_state
    }
}

impl<T: Send> WasiView for HostCtx<T> {
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

/// Internal bridge from [`HostFunctions`] to the WIT-generated `Host` traits
pub struct HostFunctionsImpl<T>(pub T);

impl<T: HostFunctions> bindings::outlayer::crypto::ed25519::Host for HostFunctionsImpl<T> {
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        self.0.ed25519_derive_public_key(path)
    }
    fn sign(&mut self, path: String, msg: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        self.0.ed25519_sign(path, msg)
    }
}

impl<T: HostFunctions> bindings::outlayer::crypto::secp256k1::Host for HostFunctionsImpl<T> {
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        self.0.secp256k1_derive_public_key(path)
    }
    fn sign(&mut self, path: String, msg: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        self.0.secp256k1_sign(path, msg)
    }
}

impl<H: HostFunctions + 'static> HasData for HostCtx<H> {
    type Data<'a> = HostFunctionsImpl<&'a mut H>;
}
