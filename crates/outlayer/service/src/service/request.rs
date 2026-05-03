use bytes::Bytes;
use defuse_outlayer_vm_runner::host::primitives::AppId;

pub struct Request<'a> {
    pub app_id: AppId<'a>,
    pub input: Bytes,
    pub fuel: u64,
}

pub enum App<'a> {
    AppId(AppId<'a>),
    Inline()
}
