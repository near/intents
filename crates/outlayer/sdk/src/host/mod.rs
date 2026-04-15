#[cfg(target_family = "wasm")]
mod sys;

use std::sync::LazyLock;

#[cfg(target_family = "wasm")]
type Host = sys::SysHost;

#[cfg(not(target_family = "wasm"))]
type Host = defuse_outlayer_worker_host::WorkerHost;

pub static HOST: LazyLock<Host> = LazyLock::new(|| {
    #[cfg(target_family = "wasm")]
    {
        sys::SysHost
    }
    #[cfg(not(target_family = "wasm"))]
    {
        unimplemented!("init_host is not implemented");
    }
});
