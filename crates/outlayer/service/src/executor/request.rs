use bytes::Bytes;
use defuse_outlayer_vm_runner::{host::AppContext, wasmtime::component::Component};

pub struct Request {
    pub ctx: Context,
    pub component: Component,
    pub limits: Limits,
}

pub struct Context {
    pub app: AppContext,
    pub input: Bytes,
}

pub struct Limits {
    pub stdout_size: usize,
    pub stderr_size: usize,
    pub fuel: u64,
}
