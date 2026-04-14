#[cfg(not(target_family = "wasm"))]
mod native;
#[cfg(target_family = "wasm")]
mod sys;

use std::sync::OnceLock;

pub use defuse_outlayer_host as host;

#[cfg(target_family = "wasm")]
type Host = sys::SysHost;

#[cfg(not(target_family = "wasm"))]
type Host = defuse_outlayer_worker_host::WorkerHost;

static HOST: OnceLock<Host> = OnceLock::new();

/// The `host` function provides access to the host functionality.
/// Usage:
/// ```rust
/// let pk = host().ed25519_derive_public_key("some/path");
/// ```
pub fn host() -> &'static Host {
    HOST.get_or_init(|| {
        #[cfg(target_family = "wasm")]
        {
            sys::SysHost
        }
        #[cfg(not(target_family = "wasm"))]
        {
            native::init_host()
        }
    })
}
