mod http;
mod inline;

pub use http::HttpResolver;
pub use inline::InlineResolver;

use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::future::BoxFuture;
use lru::LruCache;
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

// ── Scheme-routing + hash-verified + cached resolver ─────────────────────────

/// Resolves a WASM binary by URL, verifies its SHA-256 hash, and caches the
/// result keyed by `(url, hash)` so that the same binary is never fetched twice.
///
/// Accepts `(url, expected_hash)`. Returns an error if the fetched bytes do
/// not match `expected_hash` (`ResolveError::HashMismatch`).
///
/// TODO: add a TTL to cache entries. Without TTL, if a WASM binary at a URL is
/// silently replaced but the caller keeps sending the old hash, the cache will
/// keep returning the previously-verified bytes indefinitely. A TTL forces
/// eventual re-fetch and re-verification, surfacing the hash mismatch rather
/// than serving stale content forever.
pub struct ResolverService {
    cache: Arc<Mutex<LruCache<(String, [u8; 32]), Bytes>>>,
    inner: BoxCloneService<String, Bytes, ResolveError>,
}

impl Clone for ResolverService {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl Service<(String, [u8; 32])> for ResolverService {
    type Response = Bytes;
    type Error = ResolveError;
    type Future = BoxFuture<'static, Result<Bytes, ResolveError>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ResolveError>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, (url, expected_hash): (String, [u8; 32])) -> Self::Future {
        let cache = self.cache.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let cache_key = (url.clone(), expected_hash);
            if let Some(cached) = cache.lock().unwrap().get(&cache_key).cloned() {
                return Ok(cached);
            }

            let bytes = inner.call(url).await?;

            let actual: [u8; 32] = Sha256::digest(&bytes).into();
            if actual != expected_hash {
                return Err(ResolveError::HashMismatch {
                    expected: hex_encode(&expected_hash),
                    actual:   hex_encode(&actual),
                });
            }

            cache.lock().unwrap().put(cache_key, bytes.clone());
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

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    })
}
