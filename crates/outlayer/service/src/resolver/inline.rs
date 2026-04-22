use std::task::{Context, Poll};

use base64::{engine::general_purpose::STANDARD, Engine};
use bytes::Bytes;
use futures_util::future::BoxFuture;
use tower::Service;

use super::ResolveError;

/// Decodes `data:[mediatype];base64,<data>` URLs, returning the raw bytes.
#[derive(Clone)]
pub struct InlineResolver {
    max_bytes: usize,
}

impl InlineResolver {
    pub const fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }
}

impl Service<String> for InlineResolver {
    type Response = Bytes;
    type Error = ResolveError;
    type Future = BoxFuture<'static, Result<Bytes, ResolveError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), ResolveError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, url: String) -> Self::Future {
        let max_bytes = self.max_bytes;
        Box::pin(async move { decode_data_url(&url, max_bytes) })
    }
}

fn decode_data_url(url: &str, max_bytes: usize) -> Result<Bytes, ResolveError> {
    let rest = url
        .strip_prefix("data:")
        .ok_or_else(|| ResolveError::InlineDecode("not a data URL".into()))?;
    let comma = rest
        .find(',')
        .ok_or_else(|| ResolveError::InlineDecode("missing comma separator".into()))?;
    let header = &rest[..comma];
    let data = &rest[comma + 1..];
    if !header.ends_with(";base64") {
        return Err(ResolveError::InlineDecode("only base64 encoding is supported".into()));
    }
    let bytes = STANDARD
        .decode(data)
        .map(Bytes::from)
        .map_err(|e| ResolveError::InlineDecode(e.to_string()))?;
    if bytes.len() > max_bytes {
        return Err(ResolveError::TooLarge { size: bytes.len(), limit: max_bytes });
    }
    Ok(bytes)
}
