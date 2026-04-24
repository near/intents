use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use lru::LruCache;
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
            max_fetch_bytes: 10 * 1024 * 1024, // 10 MiB
        }
    }
}

#[derive(Clone)]
pub struct CacheLayer<K, V> {
    cache: Arc<Mutex<LruCache<K, V>>>,
}

impl<K: Hash + Eq, V> CacheLayer<K, V> {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }
}

impl<S, K: Hash + Eq, V> Layer<S> for CacheLayer<K, V> {
    type Service = CacheService<S, K, V>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService {
            inner,
            cache: self.cache.clone(),
        }
    }
}

#[derive(Clone)]
pub struct CacheService<S, K, V> {
    inner: S,
    cache: Arc<Mutex<LruCache<K, V>>>,
}

impl<S, K, V> Service<K> for CacheService<S, K, V>
where
    S: Service<K, Response = V> + Clone + Send + 'static,
    S::Future: Send,
    K: Hash + Eq + Clone + Send + 'static,
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

        Box::pin(async move {
            let cached = cache.lock().unwrap().get(&key).cloned();
            if let Some(cached) = cached {
                return Ok(cached);
            }
            let value = inner.call(key.clone()).await?;
            cache.lock().unwrap().put(key, value.clone());
            Ok(value)
        })
    }
}
