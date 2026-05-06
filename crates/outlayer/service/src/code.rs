use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use bytes::Bytes;
use defuse_outlayer_primitives::AppId;
use url::Url;

use crate::HashedCode;

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
            Self::Inline { code } => {
                AppCodeUrl::from_code(HashedCode::new(code.clone())).immutable_app_id()
            }
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
            Self::Url(url) => url.immutable_app_id(),
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
    pub fn from_code(code: impl Into<HashedCode>) -> Self {
        let code = code.into();
        Self {
            code_url: format!(
                "data:application/wasm;base64,{}",
                URL_SAFE.encode(code.bytes())
            )
            .parse()
            .expect("URL: parse"),
            code_hash: code.hash(),
        }
    }

    pub fn immutable_app_id(&self) -> AppId<'static> {
        // TODO: derive from state_init
        todo!()
    }
}
