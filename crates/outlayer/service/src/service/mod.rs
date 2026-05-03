mod error;
mod request;

use bytes::Bytes;
use defuse_outlayer_vm_runner::host::{Context as HostContext, primitives::AppId};
use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use tower::Service;

use crate::{
    executor::{self, Executor},
    resolver::App,
};

pub use self::{error::*, request::*};

pub struct Outlayer<R> {
    executor: Executor,
    resolver: R,
}

impl<'a, Resolver> Service<Request<'a>> for Outlayer<Resolver>
where
    Resolver: Service<AppId<'a>, Response = Bytes, Error = anyhow::Error, Future: Send + 'a> + Send,
{
    type Response = executor::Response;

    type Error = Error;

    type Future = BoxFuture<'a, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, req: Request<'a>) -> Self::Future {
        match req.app {
            App::Inline { wasm } => futures::future::ready(Ok(wasm)).boxed(),
            App::AppId(app_id) => self.resolver.call(app_id).boxed(),
        }
        .map_err(Error::Resolver)
        .and_then(move |wasm| {
            self.executor
                .call(executor::Request {
                    ctx: executor::Context {
                        input: req.input,
                        host: HostContext {
                            app_id: req.app.into_app_id(),
                        },
                    },
                    wasm,
                    fuel: req.fuel,
                })
                .map_err(Error::Executor)
        })
        .boxed()
    }
}
