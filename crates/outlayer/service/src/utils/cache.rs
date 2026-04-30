use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use lru::LruCache;
use sha2::{Digest, Sha256};
use tower::{Layer, Service};

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub capacity: NonZeroUsize,
    pub max_fetch_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            capacity: NonZeroUsize::new(100).unwrap(),
            max_fetch_bytes: 100 * 1024 * 1024, // 10 MiB
        }
    }
}

fn cache_key<K: Hash>(key: &K) -> [u8; 32] {
    struct Sha256Hasher(Sha256);
    impl std::hash::Hasher for Sha256Hasher {
        fn write(&mut self, bytes: &[u8]) {
            self.0.update(bytes);
        }
        fn finish(&self) -> u64 {
            0 // never called
        }
    }
    let mut h = Sha256Hasher(Sha256::new());
    key.hash(&mut h);
    h.0.finalize().into()
}

#[derive(Clone)]
pub struct CacheLayer<K, V> {
    cache: Arc<Mutex<LruCache<[u8; 32], V>>>,
    _phantom: PhantomData<K>,
}

impl<K: Hash, V> CacheLayer<K, V> {
    pub const fn new(cache: Arc<Mutex<LruCache<[u8; 32], V>>>) -> Self {
        Self {
            cache,
            _phantom: PhantomData,
        }
    }
}

impl<S, K: Hash, V> Layer<S> for CacheLayer<K, V> {
    type Service = CacheService<S, K, V>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService {
            inner,
            cache: self.cache.clone(),
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct CacheService<S, K, V> {
    inner: S,
    cache: Arc<Mutex<LruCache<[u8; 32], V>>>,
    _phantom: PhantomData<K>,
}

impl<S, K, V> Service<K> for CacheService<S, K, V>
where
    S: Service<K, Response = V> + Clone + Send + 'static,
    S::Future: Send,
    K: Hash + Send + 'static,
    V: Clone + Send + 'static,
{
    type Response = V;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<V, S::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, key: K) -> Self::Future {
        let cache = self.cache.clone();
        let mut inner = self.inner.clone();
        let hash = cache_key(&key);

        Box::pin(async move {
            let cached = cache.lock().unwrap().get(&hash).cloned();
            if let Some(cached) = cached {
                return Ok(cached);
            }
            let value = inner.call(key).await?;
            cache.lock().unwrap().put(hash, value.clone());
            Ok(value)
        })
    }
}
