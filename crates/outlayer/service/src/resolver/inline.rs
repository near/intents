use std::sync::Arc;
use std::task::{Context, Poll};

use base64::{Engine, engine::general_purpose::STANDARD};
use bytes::Bytes;
use futures_util::future::BoxFuture;
use tower::Service;
use tracing::Instrument as _;

use super::ResolveError;

const DATA_URL_PREFIX: &str = "data:application/wasm;base64,";

/// Decodes `data:application/wasm;base64,<data>` URLs, returning the raw bytes.
#[derive(Clone)]
pub struct InlineResolver;

impl Service<String> for InlineResolver {
    type Response = Arc<Bytes>;
    type Error = ResolveError;
    type Future = BoxFuture<'static, Result<Arc<Bytes>, ResolveError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), ResolveError>> {
        Poll::Ready(Ok(()))
    }

    #[tracing::instrument(level = "debug", name = "inline.decode", skip_all)]
    fn call(&mut self, url: String) -> Self::Future {
        Box::pin(async move {
            let data = url.strip_prefix(DATA_URL_PREFIX).ok_or_else(|| {
                ResolveError::InlineDecode(format!("expected prefix `{DATA_URL_PREFIX}`"))
            })?;
            let bytes = STANDARD
                .decode(data)
                .map(|b| Arc::new(Bytes::from(b)))
                .map_err(|e| ResolveError::InlineDecode(e.to_string()))?;
            tracing::debug!(bytes = bytes.len(), "decoded inline wasm data URL");
            Ok(bytes)
        }.instrument(tracing::Span::current()))
    }
}
