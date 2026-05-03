use bytes::Bytes;
use defuse_outlayer_vm_runner::host::primitives::AppId;

pub enum App<'a> {
    // TODO: feature flag?
    Inline { wasm: Bytes },
    AppId(AppId<'a>),
}

impl<'a> App<'a> {
    pub fn into_app_id(self) -> AppId<'a> {
        match self {
            Self::AppId(app_id) => app_id,
            Self::Inline { wasm } => {
                // TODO: derive from state_init
                todo!()
            }
        }
    }
}
