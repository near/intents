use std::io;

use bytes::Bytes;
use futures::TryStreamExt as _;
use reqwest::Client;
use tokio::io::AsyncReadExt as _;
use tokio_util::io::StreamReader;
use url::Url;

pub struct HttpResolver {
    // TODO: caching?
    client: Client,
    max_len: usize,
}

impl HttpResolver {
    pub const fn new(client: Client, max_len: usize) -> Self {
        Self { client, max_len }
    }

    pub async fn resolve(&self, url: Url) -> Result<Bytes, Error> {
        let resp = self
            .client
            .get(url)
            .header("Accept", "application/wasm,application/octet-stream")
            // TODO: + Accept-Encoding?
            .send()
            .await?
            .error_for_status()?;

        // Fast-reject on a declared Content-Length that doesn't fit in `usize` or
        // already exceeds the limit, and pre-allocate to the declared length otherwise.
        let mut buf = match resp.content_length().map(usize::try_from) {
            Some(Ok(len)) if len <= self.max_len => Vec::with_capacity(len),
            Some(_) => {
                return Err(Error::TooLarge {
                    limit: self.max_len,
                });
            }
            None => Vec::new(),
        };

        let limit = u64::try_from(self.max_len).expect("usize fits in u64");
        let stream = resp.bytes_stream().map_err(io::Error::other);
        let mut stream = StreamReader::new(stream).take(limit);
        stream.read_to_end(&mut buf).await?;

        if stream.into_inner().read(&mut [0u8]).await? != 0 {
            return Err(Error::TooLarge {
                limit: self.max_len,
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
