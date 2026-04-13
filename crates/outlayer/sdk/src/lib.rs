mod host;

pub use defuse_outlayer_types as types;

#[cfg(not(target_family = "wasm"))]
pub use defuse_outlayer_host as workers;

#[cfg(target_family = "wasm")]
pub type Host = host::sys::SysHost;

#[cfg(not(target_family = "wasm"))]
pub type Host = defuse_outlayer_host::WorkerHost;
