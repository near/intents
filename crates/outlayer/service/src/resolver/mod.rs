mod http;
mod inline;

pub use http::HttpResolver;
pub use inline::InlineResolver;

use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::future::BoxFuture;
use sha2::{Digest, Sha256};
use thiserror::Error;
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

// ── Scheme-routing + hash-verified resolver ───────────────────────────────────

/// Verifies the SHA-256 hash of the fetched WASM binary.
///
/// Accepts `(url, expected_hash)`. Returns an error if the fetched bytes do
/// not match `expected_hash` (`ResolveError::HashMismatch`).
///
/// Caching is the caller's responsibility — apply a `CacheLayer` on top.
#[derive(Clone)]
pub struct ResolverService {
    inner: BoxCloneService<String, Arc<Bytes>, ResolveError>,
}

impl ResolverService {
    pub const fn new(inner: BoxCloneService<String, Arc<Bytes>, ResolveError>) -> Self {
        Self { inner }
    }
}

impl Service<(String, [u8; 32])> for ResolverService {
    type Response = Arc<Bytes>;
    type Error = ResolveError;
    type Future = BoxFuture<'static, Result<Arc<Bytes>, ResolveError>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ResolveError>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, (url, expected_hash): (String, [u8; 32])) -> Self::Future {
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let bytes = inner.call(url).await?;

            let actual: [u8; 32] = Sha256::digest(bytes.as_ref()).into();
            if actual != expected_hash {
                return Err(ResolveError::HashMismatch {
                    expected: hex_encode(&expected_hash),
                    actual:   hex_encode(&actual),
                });
            }

            Ok(bytes)
        })
    }
}

/// Builds a scheme-routing resolver.
///
/// URL scheme dispatch (via the `url` crate):
///   `http` / `https` → `HttpResolver`
///   `data`           → `InlineResolver`
///   anything else    → `ResolveError::UnsupportedScheme`
///
/// Both resolvers reject responses larger than `max_bytes`.
pub fn build_resolver(max_bytes: usize) -> BoxCloneService<String, Arc<Bytes>, ResolveError> {
    let unsupported = service_fn(|url: String| async move {
        Err::<Arc<Bytes>, ResolveError>(ResolveError::UnsupportedScheme(url))
    });

    let steer = Steer::new(
        vec![
            BoxCloneService::new(HttpResolver::new(reqwest::Client::new(), max_bytes)),
            BoxCloneService::new(InlineResolver),
            BoxCloneService::new(unsupported),
        ],
        |url: &String, _: &[BoxCloneService<String, Arc<Bytes>, ResolveError>]| -> usize {
            Url::parse(url).map_or(2, |u| match u.scheme() {
                "http" | "https" => 0,
                "data" => 1,
                _ => 2,
            })
        },
    );

    BoxCloneService::new(steer)
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    })
}
