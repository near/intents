use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::future::BoxFuture;
use futures_util::TryStreamExt;
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;
use tower::Service;

use super::ResolveError;

#[derive(Clone)]
pub struct HttpResolver {
    client:    reqwest::Client,
    max_bytes: usize,
}

impl HttpResolver {
    pub const fn new(client: reqwest::Client, max_bytes: usize) -> Self {
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

    fn call(&mut self, url: String) -> Self::Future {
        let client = self.client.clone();
        let max_bytes = self.max_bytes;
        Box::pin(async move {
            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| ResolveError::Http(e.to_string()))?;

            let stream = response
                .bytes_stream()
                .map_err(std::io::Error::other);
            let mut reader = StreamReader::new(stream);

            let mut buf = Vec::new();
            let limit = u64::try_from(max_bytes).unwrap_or(u64::MAX);
            (&mut reader).take(limit).read_to_end(&mut buf).await
                .map_err(|e| ResolveError::Http(e.to_string()))?;

            if reader.into_inner().try_next().await
                .map_err(|e| ResolveError::Http(e.to_string()))?
                .is_some()
            {
                return Err(ResolveError::TooLarge { size: buf.len() + 1, limit: max_bytes });
            }

            Ok(Arc::new(Bytes::from(buf)))
        })
    }
}
