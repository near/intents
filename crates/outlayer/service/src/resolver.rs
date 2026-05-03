use bytes::Bytes;
use defuse_outlayer_primitives::AppId;

pub trait Resolver {
    type Error;

    fn resolve_wasm(
        &self,
        app_id: AppId<'_>,
    ) -> impl Future<Output = Result<Bytes, Self::Error>> + '_;
}
