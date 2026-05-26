mod near;
mod url;
mod config;

use crate::{AppCodeUrl, CodeRef};
use bytes::Bytes;
use defuse_outlayer_primitives::AppId;
use sha2::{Digest, Sha256};

pub use self::config::{ResolverBuilder, ResolverConfig};
pub use self::near::NearResolver;
pub use self::url::{HttpResolver, UrlResolver};


#[derive(Clone)]
pub struct Resolver {
    near: NearResolver,
    url: UrlResolver,
}

impl Resolver {
    pub const fn new(near: NearResolver, url: UrlResolver) -> Self {
        Self { near, url }
    }

    pub async fn resolve_code_url(&self, code: CodeRef<'_>) -> Result<AppCodeUrl, Error> {
        match code {
            CodeRef::AppId(app_id) => match app_id {
                AppId::Near(oa_contract_id) => {
                    self.near.resolve(oa_contract_id).await.map_err(Into::into)
                }
            },
            CodeRef::Url(app_code_url) => Ok(app_code_url),
        }
    }

    pub async fn resolve_code(
        &self,
        AppCodeUrl {
            code_url,
            code_hash,
        }: AppCodeUrl,
    ) -> Result<Bytes, Error> {
        let code = self.url.resolve(code_url).await?;
        let actual = Sha256::digest(&code);
        if actual != code_hash {
            return Err(Error::CodeHashMismatch);
        }
        Ok(code)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("code hash mismatch")]
    CodeHashMismatch,

    #[error("NEAR: {0}")]
    NearRpc(#[from] near_kit::Error),

    #[error("URL: {0}")]
    Url(#[from] self::url::Error),
}
