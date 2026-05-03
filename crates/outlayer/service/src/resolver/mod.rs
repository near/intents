mod near;
mod url;

use bytes::Bytes;
use defuse_outlayer_primitives::AppId;
use sha2::{Digest, Sha256};

use crate::{AppCodeUrl, CodeRef};

use self::{near::NearResolver, url::UrlResolver};

// TODO: caching
pub struct Resolver {
    near: NearResolver,
    url: UrlResolver,
}

impl Resolver {
    pub async fn resolve_code(&self, code: CodeRef<'_>) -> Result<Bytes> {
        let AppCodeUrl {
            code_url,
            code_hash,
        } = match code {
            CodeRef::AppId(app_id) => self.resolve_app_id(app_id).await?,
            CodeRef::Url(code_url) => code_url,
        };

        let code = self.url.resolve(code_url).await?;

        if code_hash != Sha256::digest(&code) {
            return Err(Error::CodeHashMismatch);
        }

        Ok(code)
    }

    async fn resolve_app_id(&self, app_id: AppId<'_>) -> Result<AppCodeUrl> {
        match app_id {
            AppId::Near(oa_contract_id) => {
                self.near.resolve(oa_contract_id).await.map_err(Into::into)
            }
        }
    }
}

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("code hash mismatch")]
    CodeHashMismatch,

    #[error("NEAR: {0}")]
    NearRpc(#[from] near_kit::Error),

    #[error("URL: {0}")]
    Url(#[from] self::url::Error),
}
