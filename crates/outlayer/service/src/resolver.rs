use std::task::{Context, Poll};

use base64::{engine::general_purpose::STANDARD, Engine};
use bytes::Bytes;
use futures_util::future::BoxFuture;
use futures_util::TryStreamExt;
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;
use tower::steer::Steer;
use tower::util::BoxCloneService;
use tower::{service_fn, Service};
use url::Url;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("HTTP fetch failed: {0}")]
    Http(String),
    #[error("fetch timed out")]
    Timeout,
    #[error("invalid data URL: {0}")]
    InlineDecode(String),
    #[error("unsupported URL scheme: {0}")]
    UnsupportedScheme(String),
    #[error("response too large: {size} bytes exceeds limit of {limit} bytes")]
    TooLarge { size: usize, limit: usize },
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
}

// ── HTTP resolver ─────────────────────────────────────────────────────────────

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
    type Response = Bytes;
    type Error = ResolveError;
    type Future = BoxFuture<'static, Result<Bytes, ResolveError>>;

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

            Ok(Bytes::from(buf))
        })
    }
}

// ── Inline (data: URL) resolver ───────────────────────────────────────────────

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

// ── Steer-based router ────────────────────────────────────────────────────────

/// Builds a scheme-routing resolver backed by `Steer`.
///
/// URL scheme dispatch (via the `url` crate):
///   `http` / `https` → `HttpResolver`
///   `data`           → `InlineResolver`
///   anything else    → `ResolveError::UnsupportedScheme`
///
/// Both resolvers reject responses larger than `max_bytes`.
pub fn build_resolver(max_bytes: usize) -> BoxCloneService<String, Bytes, ResolveError> {
    let unsupported = service_fn(|url: String| async move {
        Err::<Bytes, ResolveError>(ResolveError::UnsupportedScheme(url))
    });

    let steer = Steer::new(
        vec![
            BoxCloneService::new(HttpResolver::new(reqwest::Client::new(), max_bytes)),
            BoxCloneService::new(InlineResolver::new(max_bytes)),
            BoxCloneService::new(unsupported),
        ],
        |url: &String, _: &[BoxCloneService<String, Bytes, ResolveError>]| -> usize {
            Url::parse(url).map_or(2, |u| match u.scheme() {
                "http" | "https" => 0,
                "data" => 1,
                _ => 2,
            })
        },
    );

    BoxCloneService::new(steer)
}
