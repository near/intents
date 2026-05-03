use bytes::Bytes;
use defuse_outlayer_vm_runner::{host::Context as HostContext, wasmtime::component::Component};

pub struct Request {
    pub ctx: Context,
    // TODO: replace with binary and add caching layer?
    pub component: Component,
    pub fuel: u64,
}

pub struct Context {
    pub input: Bytes,
    pub host: HostContext<'static>,
}
