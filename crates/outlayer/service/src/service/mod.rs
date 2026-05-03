mod error;
mod request;

use std::{
    sync::{Arc, Mutex},
    task::{self, ready},
};

use bytes::Bytes;
use defuse_outlayer_vm_runner::host::{Context as HostContext, primitives::AppId};
use futures::{FutureExt, future::BoxFuture};
use tower::{Service, ServiceExt, buffer::Buffer};

use crate::{
    executor::{self, Executor},
    resolver::App,
};

pub use self::{error::*, request::*};

pub struct Outlayer<R> {
    resolver: R,
    executor: Buffer<executor::Request, <Executor as Service<executor::Request>>::Future>,
}

impl<'a, R> Service<Request<'a>> for Outlayer<R>
where
    R: Service<AppId<'a>, Response = Bytes, Error = anyhow::Error, Future: Send + 'a> + Send,
{
    type Response = executor::Response;

    type Error = Error;

    type Future = BoxFuture<'a, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        ready!(self.resolver.poll_ready(cx));
        // ready!(self.executor.poll_ready(cx));
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<'a>) -> Self::Future {
        let app_id = req.app.app_id().into_owned();
        let wasm = match req.app {
            App::Inline { wasm } => futures::future::ready(Ok(wasm)).boxed(),
            App::AppId(app_id) => self.resolver.call(app_id).boxed(),
        };

        let executor = self.executor.clone();
        async move {
            let wasm = wasm.await.map_err(Error::Resolve)?;

            executor
                .oneshot(executor::Request {
                    ctx: executor::Context {
                        input: req.input,
                        host: HostContext { app_id },
                    },
                    wasm,
                    fuel: req.fuel,
                })
                .await
                .map_err(Error::Execute)
        }
        .boxed()
    }
}
