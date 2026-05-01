use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::TryStreamExt as _;
use futures_util::future::BoxFuture;
use tokio::io::AsyncReadExt as _;
use tokio_util::io::StreamReader;
use tower::Service;
use tracing::Instrument as _;

use super::ResolveError;

#[derive(Clone)]
pub struct HttpResolver {
    client: reqwest::Client,
    max_bytes: u64,
}

impl HttpResolver {
    pub const fn new(client: reqwest::Client, max_bytes: u64) -> Self {
        Self { client, max_bytes }
    }
}

impl Service<String> for HttpResolver {
    type Response = Arc<Bytes>;
    type Error = ResolveError;
    type Future = BoxFuture<'static, Result<Arc<Bytes>, ResolveError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), ResolveError>> {
        Poll::Ready(Ok(()))
    }

    #[tracing::instrument(level = "debug", name = "fetch::http", skip_all)]
    fn call(&mut self, url: String) -> Self::Future {
        let client = self.client.clone();
        let max_bytes = self.max_bytes;
        Box::pin(
            async move {
                let response = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| ResolveError::Http(e.to_string()))?;
                tracing::debug!(status = %response.status(), "http response received");

                let content_length = response.content_length();

                if let Some(len) = content_length {
                    if len > max_bytes {
                        return Err(ResolveError::TooLarge {
                            size: len,
                            limit: max_bytes,
                        });
                    }
                }

                // Cap reads to prevent unbounded allocation and OOM on large/malicious responses.
                // Pre-allocate to the declared content length but cap at max_bytes — a lying
                // server could advertise Content-Length: max_bytes to force a large allocation
                // even if it delivers almost no data.
                // Read max_bytes+1 so the post-read length check can detect truncation.
                let cap = usize::try_from(content_length.unwrap_or(0).min(max_bytes)).unwrap_or(0);
                let mut buf = Vec::with_capacity(cap);
                StreamReader::new(response.bytes_stream().map_err(std::io::Error::other))
                    .take(max_bytes + 1)
                    .read_to_end(&mut buf)
                    .await
                    .map_err(|e| ResolveError::Http(e.to_string()))?;

                let read = u64::try_from(buf.len()).unwrap_or(u64::MAX); // infallible: usize <= u64
                if read > max_bytes {
                    return Err(ResolveError::TooLarge {
                        size: read,
                        limit: max_bytes,
                    });
                }

                tracing::debug!(bytes = buf.len(), "http fetch complete");
                Ok(Arc::new(Bytes::from(buf)))
            }
            .instrument(tracing::Span::current()),
        )
    }
}
