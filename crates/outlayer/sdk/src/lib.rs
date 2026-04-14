mod hosts;

pub use defuse_outlayer_host as host;

#[cfg(target_family = "wasm")]
pub type Host = host::sys::SysHost;

#[cfg(not(target_family = "wasm"))]
pub type Host = defuse_outlayer_worker_host::WorkerHost;
