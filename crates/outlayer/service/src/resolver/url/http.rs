use std::{io, sync::Arc, time::Duration};

use bytes::Bytes;
use futures::TryStreamExt as _;
use moka::future::Cache;
use reqwest::Client;
use tokio::io::AsyncReadExt as _;
use tokio_util::io::StreamReader;
use url::Url;

const INITIAL_CAP: usize = 1024 * 1024;
#[derive(Clone)]
pub struct HttpResolver {
    client: Client,
    max_body_bytes: usize,
    cache: Cache<Url, Bytes>,
}

impl HttpResolver {
    pub fn new(max_body_bytes: usize, cache_ttl: Duration) -> Self {
        Self {
            client: Client::new(),
            max_body_bytes,
            cache: Cache::builder().time_to_live(cache_ttl).build(),
        }
    }

    pub async fn resolve(&self, url: Url) -> Result<Bytes, Arc<Error>> {
        self.cache.try_get_with(url.clone(), self.fetch(url)).await
    }

    async fn fetch(&self, url: Url) -> Result<Bytes, Error> {
        // TODO: have a whitelist or blacklist of domains?
        let resp = self
            .client
            .get(url)
            .header("Accept", "application/wasm,application/octet-stream")
            // TODO: + Accept-Encoding?
            .send()
            .await?
            .error_for_status()?;

        let mut buf = Vec::with_capacity({
            // Fast-reject on a declared Content-Length that doesn't fit in
            // `usize` or already exceeds the limit. Otherwise pre-allocate
            // up to a small cap to avoid attacker-controlled speculative
            // allocations from a lying server.
            let cap = resp
                .content_length()
                .map(usize::try_from)
                .transpose()
                .map_err(|_| Error::TooLarge {
                    limit: self.max_body_bytes,
                })?
                .unwrap_or_default();
            if cap > self.max_body_bytes {
                return Err(Error::TooLarge {
                    limit: self.max_body_bytes,
                });
            }
            cap.min(INITIAL_CAP)
        });

        let limit = u64::try_from(self.max_body_bytes).expect("usize fits in u64");
        let stream = resp.bytes_stream().map_err(io::Error::other);
        let mut stream = StreamReader::new(stream).take(limit);
        stream.read_to_end(&mut buf).await?;

        // `take(limit)` succeeds on truncation; read from the underlying
        // stream to verify it is actually exhausted at the cap.
        if stream.into_inner().read(&mut [0u8]).await? != 0 {
            return Err(Error::TooLarge {
                limit: self.max_body_bytes,
            });
        }

        Ok(Bytes::from(buf))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("response too large, limit is {limit} bytes")]
    TooLarge { limit: usize },
}
