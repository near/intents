use std::{error::Error, num::NonZeroUsize};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields, default))]
pub struct WorkerPoolConfig {
    pub buffer: NonZeroUsize,
    pub concurrency: usize,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            buffer: NonZeroUsize::new(1).unwrap(),
            concurrency: 3,
        }
    }
}

pub fn wrap_with_pool<S, Req>(
    svc: S,
    config: WorkerPoolConfig,
) -> tower::util::BoxCloneService<Req, S::Response, Box<dyn Error + Send + Sync>>
where
    S: tower::Service<Req> + Clone + Send + 'static,
    S::Response: 'static,
    S::Future: Send,
    S::Error: Into<Box<dyn Error + Send + Sync>> + Send + Sync,
    Req: Send + 'static,
{
    tower::util::BoxCloneService::new(
        tower::ServiceBuilder::new()
            .buffer(config.buffer.get())
            .concurrency_limit(config.concurrency)
            .service(svc),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::{Service as _, ServiceExt as _};

    #[tokio::test]
    async fn wrap_with_pool_calls_inner_service() {
        let svc = tower::service_fn(|x: i32| async move {
            Ok::<i32, std::convert::Infallible>(x * 2)
        });
        let mut wrapped = wrap_with_pool(
            svc,
            WorkerPoolConfig {
                buffer: NonZeroUsize::new(4).unwrap(),
                concurrency: 2,
            },
        );
        let result = wrapped.ready().await.unwrap().call(21).await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn wrap_with_pool_handles_multiple_calls() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter2 = counter.clone();
        let svc = tower::service_fn(move |x: usize| {
            let counter = counter2.clone();
            async move {
                counter.fetch_add(x, std::sync::atomic::Ordering::SeqCst);
                Ok::<_, std::convert::Infallible>(())
            }
        });
        let wrapped = wrap_with_pool(
            svc,
            WorkerPoolConfig {
                buffer: NonZeroUsize::new(4).unwrap(),
                concurrency: 2,
            },
        );
        // Three sequential calls via oneshot
        tower::ServiceExt::oneshot(wrapped.clone(), 1).await.unwrap();
        tower::ServiceExt::oneshot(wrapped.clone(), 2).await.unwrap();
        tower::ServiceExt::oneshot(wrapped.clone(), 3).await.unwrap();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 6);
    }
}
