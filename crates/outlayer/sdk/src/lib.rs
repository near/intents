mod host;

#[cfg(target_family = "wasm")]
pub type Host = host::sys::SysHost;

#[cfg(not(target_family = "wasm"))]
pub type Host = defuse_outlayer_host::WorkerHost;
