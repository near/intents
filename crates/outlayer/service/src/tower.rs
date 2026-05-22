use std::{error::Error, num::NonZeroUsize};

#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields, default))]
pub struct TowerConfig {
    pub buffer: NonZeroUsize,
    pub concurrency: usize,
}

impl Default for TowerConfig {
    fn default() -> Self {
        Self {
            buffer: NonZeroUsize::new(1).unwrap(),
            concurrency: 1,
        }
    }
}

pub fn wrap_with_pool<S, Req>(
    svc: S,
    config: TowerConfig,
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
    use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};

    use super::*;
    use tokio::time::{Duration, timeout};
    use tower::{Service as _, ServiceExt as _};

    const TIMEOUT: Duration = Duration::from_millis(50);

    #[tokio::test]
    async fn wrap_with_pool_calls_inner_service() {
        let svc =
            tower::service_fn(|x: i32| async move { Ok::<i32, std::convert::Infallible>(x * 2) });
        let mut wrapped = wrap_with_pool(
            svc,
            TowerConfig {
                buffer: NonZeroUsize::new(4).unwrap(),
                concurrency: 2,
            },
        );
        let result = wrapped.ready().await.unwrap().call(21).await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_wrapper_properly_allocates_slots() {
        let service = wrap_with_pool(
            tower::service_fn(|_: ()| async move { Ok::<_, std::convert::Infallible>(()) }),
            Default::default(),
        );

        let mut svc_handle1 = service.clone();
        let mut svc_handle2 = service.clone();

        svc_handle1.ready().await.unwrap();

 
        assert!(timeout(TIMEOUT, svc_handle2.ready()).await.is_err());
  }

    #[tokio::test]
    async fn test_concurency_is_handled_properly() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_handle = Arc::clone(&counter);
        let service = wrap_with_pool(
            tower::service_fn(move |_: ()| {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    Ok::<_, std::convert::Infallible>(())
                }
            }),
            TowerConfig {
                buffer: NonZeroUsize::new(4).unwrap(),
                concurrency: 2,
            },
        );

        let mut svc_handle1 = service.clone();
        let mut svc_handle2 = service.clone();
        let mut svc_handle3 = service.clone();
        let mut svc_handle4 = service.clone();
        let mut svc_handle5 = service.clone();

        svc_handle1.ready().await.unwrap();
        svc_handle2.ready().await.unwrap();
        svc_handle3.ready().await.unwrap();
        svc_handle4.ready().await.unwrap();
        assert!(timeout(TIMEOUT, svc_handle5.ready()).await.is_err());

        let _ = tokio::spawn(async move { svc_handle1.call(()).await.unwrap(); });
        let _ = tokio::spawn(async move { svc_handle2.call(()).await.unwrap(); });
        let _ = tokio::spawn(async move { svc_handle3.call(()).await.unwrap(); });
        let _ = tokio::spawn(async move { svc_handle4.call(()).await.unwrap(); });

        tokio::time::sleep(TIMEOUT).await;
        assert_eq!(counter_handle.load(std::sync::atomic::Ordering::SeqCst), 2);



    }
}
