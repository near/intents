use bytes::Bytes;
use defuse_outlayer_primitives::AppId;

pub enum App {
    // TODO: feature flag?
    Inline { wasm: Bytes },
    AppId(AppId<'static>),
}

impl App {
    pub fn app_id(&self) -> AppId<'static> {
        match self {
            Self::AppId(app_id) => app_id.clone().into_owned(),
            Self::Inline { wasm } => {
                // TODO: derive from state_init
                todo!()
            }
        }
    }
}
