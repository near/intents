use bytes::Bytes;
use reqwest::{Client, Result};
use url::Url;

pub struct HttpResolver {
    client: Client,
}

impl HttpResolver {
    pub async fn resolve(&self, url: Url) -> Result<Bytes> {
        self.client
            .get(url)
            // TODO: + Accept-Encoding?
            .header("Accept", "application/wasm")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await
    }
}
