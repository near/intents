mod data;
mod http;

use bytes::Bytes;
use url::Url;

use self::http::HttpResolver;

pub struct UrlResolver {
    http: HttpResolver,
}

impl UrlResolver {
    pub async fn resolve(&self, url: Url) -> Result<Bytes, Error> {
        match url.scheme() {
            "data" => data::resolve(url).map_err(Into::into),
            "https" => self.http.resolve(url).await.map_err(Into::into),
            // TODO: ipfs?
            scheme => Err(Error::UnsupportedScheme(scheme.to_string())),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("data: {0}")]
    Data(#[from] data::Error),

    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),

    #[error("unsupported scheme: {0}")]
    UnsupportedScheme(String),
}
