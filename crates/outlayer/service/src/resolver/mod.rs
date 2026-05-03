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
    pub async fn resolve_code(&self, code: CodeRef<'_>) -> Result<Bytes, Error> {
        let AppCodeUrl {
            code_url,
            code_hash,
        } = match code {
            CodeRef::AppId(AppId::Near(contract_id)) => self.near.resolve(contract_id).await?,
            CodeRef::Url(code_url) => code_url,
        };

        let code = self.url.resolve(code_url).await?;

        if code_hash != Sha256::digest(&code) {
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
