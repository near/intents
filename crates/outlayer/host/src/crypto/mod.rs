mod ed25519;
mod secp256k1;

use std::{marker::PhantomData, sync::Arc};

use defuse_outlayer_kdf_app::AppDerivation;

use defuse_kdf::{
    BoxSchema, DerivableCurve, DeriveExt, DeriveSigner, DynDeriveSigner, Map, ed25519::Ed25519,
    secp256k1::Secp256k1,
};
use wasmtime::component::{HasData, Linker};

use crate::{Host, HostView, bindings};

pub struct AppSigner<S>(S);

impl<'a> Host<'a> {
    #[inline]
    fn app_signer(self) -> AppSigner<Map<AppDerivation<'a>, Arc<dyn Signer>>> {
        AppSigner(AppDerivation::new(self.ctx.app_id).map(self.signer))
    }
}

impl<C, P, S> DeriveSigner<C, P> for AppSigner<S>
where
    C: DerivableCurve,
    S: DeriveSigner<C, P>,
{
    type Schema<'a>
        = S::Schema<'a>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        self.0.schema()
    }

    fn public_key(&self) -> C::PublicKey {
        self.0.public_key()
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        self.0.derive_sign(path, msg)
    }
}

pub trait Signer:
    DynDeriveSigner<Ed25519, [u8; 32]> + DynDeriveSigner<Secp256k1, [u8; 32]> + Send + Sync
{
}
impl<T> Signer for T where
    T: DeriveSigner<Ed25519, [u8; 32]> + DeriveSigner<Secp256k1, [u8; 32]> + Send + Sync
{
}

impl<'l, C, P: 'l> DeriveSigner<C, P> for dyn Signer + 'l
where
    C: DerivableCurve,
    Self: DynDeriveSigner<C, P>,
{
    type Schema<'a>
        = BoxSchema<'a, P, C::Tweak>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        DynDeriveSigner::<C, P>::schema_dyn(self)
    }

    fn public_key(&self) -> C::PublicKey {
        DynDeriveSigner::<C, P>::public_key(self)
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        DynDeriveSigner::<C, P>::derive_sign(self, path, msg)
    }
}

struct HasAppSigner<S>(PhantomData<S>);

impl<S: 'static> HasData for HasAppSigner<S> {
    type Data<'a> = AppSigner<Map<AppDerivation<'a>, S>>;
}

pub(super) fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: HostView,
{
    bindings::outlayer::crypto::ed25519::add_to_linker::<T, HasAppSigner<_>>(linker, |t| {
        t.ctx().app_signer()
    })?;
    bindings::outlayer::crypto::secp256k1::add_to_linker::<T, HasAppSigner<_>>(linker, |t| {
        t.ctx().app_signer()
    })?;
    Ok(())
}
