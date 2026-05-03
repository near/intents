use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use bytes::Bytes;
use defuse_outlayer_primitives::AppId;
use sha2::{Digest, Sha256};
use url::Url;

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code<'a> {
    Ref(CodeRef<'a>),
    // TODO: feature flag?
    Inline {
        #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base64::Base64"))]
        code: Bytes,
    },
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

#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AppCodeUrl {
    pub code_url: Url,
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::hex::Hex"))]
    pub code_hash: [u8; 32],
}

impl AppCodeUrl {
    pub fn app_id(&self) -> AppId<'static> {
        // TODO: derive from state_init
        todo!()
    }
}
