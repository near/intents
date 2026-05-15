mod ed25519;
mod secp256k1;

use std::{marker::PhantomData, sync::Arc};

use defuse_outlayer_kdf_app::{
    DeriveSigner, WithAppId,
    kdf::{ed25519::Ed25519, secp256k1::Secp256k1},
};
use wasmtime::component::{HasData, Linker};

use crate::{Host, HostView, bindings};


struct HasSigner<S>(PhantomData<S>);

impl<S: 'static> HasData for HasSigner<S> {
    type Data<'a> = WithAppId<'a, S>;
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
    pub fn signer(self) -> WithAppId<'a, Arc<dyn Signer>> {
        WithAppId::new(self.ctx.app_id, self.signer)
    }
}

pub(super) fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: HostView,
{
    // TODO
    bindings::outlayer::crypto::ed25519::add_to_linker::<T, HasSigner<_>>(linker, |t| {
        t.ctx().signer()
    })?;
    bindings::outlayer::crypto::secp256k1::add_to_linker::<T, HasSigner<_>>(linker, |t| {
        t.ctx().signer()
    })?;
    Ok(())
}

// pub trait SignerView<S>: Send {
//     fn ctx(&mut self) -> WithAppId<'_, S>;
// }

// impl<'a> SignerView<&'a InMemorySigner> for Host<'a> {
//     fn ctx(&mut self) -> WithAppId<'_, &'a InMemorySigner> {
//         WithAppId::new(self.ctx.app_id.as_ref(), &self.signer)
//     }
// }

// struct HasSigner<S>(PhantomData<S>);

// impl<S> HasData for HasSigner<S> {
//     type Data<'a> = WithAppId<'a, S>;
// }

// impl<C> DeriveSigner<C, str> for Host<'_>
// where
//     C: Curve,
//     InMemorySigner: DeriveSigner<C, [u8; 32]>,
// {
//     type Scheme = <InMemorySigner as DeriveSigner<C, [u8; 32]>>::Scheme;

//     #[doc = " Get master public key of the signer"]
//     fn public_key(&self) -> C::PublicKey {
//         todo!()
//     }

//     #[doc = " Sign given message with a secret key **internally** derived for given"]
//     #[doc = " [`path`](DerivableCurve::Path)."]
//     #[doc = ""]
//     #[doc = " NOTE: the returned signatures might be non-deterministic, i.e."]
//     #[doc = " implementations MAY return different signatures for the same"]
//     #[doc = " `path` and `msg`."]
//     fn derive_sign(&self, path: &str, msg: &C::Message) -> C::Signature {
//         let tweak = self.tweak(, path);
//     }
// }

// pub struct BorshSha3_256<S: ?Sized>(PhantomData<S>);

// impl<S, P> PublicKeyDerivationScheme<P> for BorshSha3_256<S>
// where
//     S: PublicKeyDerivationScheme<[u8; 32]> + ?Sized,
//     P: BorshSerialize + ?Sized,
// {
//     type Curve = S::Curve;

//     fn derive_public_key(
//         master_pk: &<Self::Curve as Curve>::PublicKey,
//         path: &P,
//     ) -> <Self::Curve as Curve>::PublicKey {
//         let mut hasher = IoWrapper(Sha3_256::new());
//         borsh::to_writer(&mut hasher, path).expect("borsh");
//         S::derive_public_key(master_pk, &hasher.0.finalize().into())
//     }
// }
