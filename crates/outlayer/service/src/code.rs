use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use bytes::Bytes;
use defuse_outlayer_primitives::AppId;
use sha2::{Digest, Sha256};
use url::Url;

pub enum Code<'a> {
    Ref(CodeRef<'a>),
    // TODO: feature flag?
    Inline { code: Bytes },
}

impl Code<'_> {
    pub fn app_id(&self) -> AppId<'static> {
        match self {
            Self::Ref(app) => app.app_id(),
            Self::Inline { code } => AppCodeUrl {
                // See <https://developer.mozilla.org/en-US/docs/Web/URI/Reference/Schemes/data>
                code_url: format!("data:application/wasm;base64,{}", URL_SAFE.encode(code))
                    .parse()
                    .expect("URL: parse"),
                code_hash: Sha256::digest(code).into(),
            }
            .app_id(),
        }
    }
}

pub enum CodeRef<'a> {
    AppId(AppId<'a>),
    Url(AppCodeUrl),
}

impl CodeRef<'_> {
    pub fn app_id(&self) -> AppId<'static> {
        match self {
            Self::AppId(app_id) => app_id.clone().into_owned(),
            Self::Url(url) => url.app_id(),
        }
    }
}

pub struct AppCodeUrl {
    pub code_url: Url,
    pub code_hash: [u8; 32],
}

impl AppCodeUrl {
    pub fn app_id(&self) -> AppId<'static> {
        // TODO: derive from state_init
        todo!()
    }
}
