mod near;
mod url;

use crate::{AppCodeUrl, CodeRef};
use bytes::Bytes;
use defuse_outlayer_primitives::AppId;
use sha2::{Digest, Sha256};

use self::{near::NearResolver, url::UrlResolver};

#[derive(Clone)]
pub struct Resolver {
    near: NearResolver,
    url: UrlResolver,
}

impl Resolver {
    pub async fn resolve_code(&self, code: CodeRef<'_>) -> Result<Bytes> {
        match code {
            CodeRef::AppId(app_id) => {
                let url = self.resolve_app_id(app_id.as_ref()).await?;
                self.fetch_and_verify(&url).await
            }
            CodeRef::Url(app_code_url) => self.fetch_and_verify(&app_code_url).await,
        }
    }

    async fn fetch_and_verify(&self, url: &AppCodeUrl) -> Result<Bytes> {
        let code = self.url.resolve(url.code_url.clone()).await?;
        if url.code_hash != Sha256::digest(&code) {
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
