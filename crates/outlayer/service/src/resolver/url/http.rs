use bytes::Bytes;
use reqwest::{Client, Result};
use url::Url;

pub struct HttpResolver {
    // TODO: caching?
    client: Client,
}

impl HttpResolver {
    pub async fn resolve(&self, url: Url) -> Result<Bytes> {
        let resp = self
            .client
            .get(url)
            .header("Accept", "application/wasm,application/octet-stream")
            // TODO: + Accept-Encoding?
            .send()
            .await?
            .error_for_status()?;

        // TODO: check resp.content_length()

        // TODO: limit downloaded size
        resp.bytes().await
    }
}
