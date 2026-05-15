mod ed25519;
mod secp256k1;

use std::{marker::PhantomData, sync::Arc};

use defuse_outlayer_kdf_app::{
    AppSigner, DeriveSigner,
    kdf::{ed25519::Ed25519, secp256k1::Secp256k1},
};
use wasmtime::component::{HasData, Linker};

use crate::{Host, HostView, bindings};

struct HasSigner<S>(PhantomData<S>);

impl<S: 'static> HasData for HasSigner<S> {
    type Data<'a> = AppSigner<'a, S>;
}

pub trait Signer:
    DeriveSigner<Ed25519, [u8; 32]> + DeriveSigner<Secp256k1, [u8; 32]> + Send + Sync
{
}
impl<T> Signer for T where
    T: DeriveSigner<Ed25519, [u8; 32]> + DeriveSigner<Secp256k1, [u8; 32]> + Send + Sync
{
}

impl<'a> Host<'a> {
    // // TODO: no pub
    #[inline]
    pub fn app_signer(self) -> AppSigner<'a, Arc<dyn Signer>> {
        AppSigner::new(self.ctx.app_id, self.signer.clone())
    }
}

pub(super) fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: HostView,
{
    bindings::outlayer::crypto::ed25519::add_to_linker::<T, HasSigner<_>>(linker, |t| {
        t.ctx().app_signer()
    })?;
    bindings::outlayer::crypto::secp256k1::add_to_linker::<T, HasSigner<_>>(linker, |t| {
        t.ctx().app_signer()
    })?;
    Ok(())
}
