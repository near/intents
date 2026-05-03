use bytes::Bytes;
use defuse_outlayer_vm_runner::host::Context as HostContext;

pub struct Request {
    pub ctx: Context,
    pub wasm: Bytes,
    pub fuel: u64,
}

pub struct Context {
    pub input: Bytes,
    pub host: HostContext<'static>,
}
