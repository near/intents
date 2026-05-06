use bytes::Bytes;

// Safe: usize is at most 64 bits wide (pointer-width), so it always fits in u64.
const USIZE_FITS_U64: &str = "usize should always fit in u64";
use futures::TryStreamExt as _;
use reqwest::Client;
use url::Url;

pub struct HttpResolver {
    // TODO: caching?
    client: Client,
    max_len: u64,
}

impl HttpResolver {
    pub fn new(client: Client, max_len: u64) -> Self {
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

        // Fast-reject on a declared Content-Length that already exceeds the limit,
        // and pre-allocate to the declared length otherwise.
        let mut buf = match resp.content_length() {
            Some(len) if len > self.max_len => {
                return Err(Error::TooLarge {
                    limit: self.max_len,
                });
            }
            Some(len) => usize::try_from(len).map(Vec::with_capacity).unwrap_or_default(),
            None => Vec::new(),
        };

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.try_next().await? {
            let buf_len = u64::try_from(buf.len()).expect(USIZE_FITS_U64);
            let chunk_len = u64::try_from(chunk.len()).expect(USIZE_FITS_U64);
            match buf_len.checked_add(chunk_len) {
                Some(new_len) if new_len <= self.max_len => {}
                Some(_) | None => {
                    return Err(Error::TooLarge {
                        limit: self.max_len,
                    });
                }
            }
            buf.extend_from_slice(&chunk);
        }

        Ok(Bytes::from(buf))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("response too large, limit is {limit} bytes")]
    TooLarge { limit: u64 },
}
