mod ed25519;
mod secp256k1;

use std::{borrow::Cow, sync::Arc};

use defuse_outlayer_kdf_app::{
    DerivableCurve, DerivationSchema, DeriveSigner, WithAppId,
    kdf::{ed25519::Ed25519, secp256k1::Secp256k1},
};
use defuse_outlayer_signer::InMemorySigner;
use wasmtime::component::{HasData, Linker};

use crate::{Host, HostView, bindings};

// struct CryptoImpl<S>(S);

// impl<C, P, S> DeriveSigner<C, P> for CryptoImpl<S>
// where
//     C: DerivableCurve + ?Sized + 'static,
//     S: DeriveSigner<C, P>,
//     P: 'static,
// {
//     type Schema<'a>
//         = Box<dyn DerivationSchema<C, P, Output = C::Tweak> + 'a>
//     where
//         Self: 'a;

//     fn schema(&self) -> Self::Schema<'_> {
//         Box::new(self.0.schema())
//     }

//     fn public_key(&self) -> C::PublicKey {
//         self.0.public_key()
//     }

//     fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
//         self.0.derive_sign(path, msg)
//     }
// }

// trait BoxableDeriveSigner<'a, C: DerivableCurve, P>:
//     DeriveSigner<C, P, Schema<'a> = Box<dyn DerivationSchema<C, P, Output = C::Tweak> + 'a>> + 'a
// {
// }

// pub trait Signer<'a>:
//     BoxableDeriveSigner<'a, Ed25519, [u8; 32]> + BoxableDeriveSigner<'a, Secp256k1, [u8; 32]>
// {
// }
// impl<'a, T> Signer<'a> for T where
//     T: BoxableDeriveSigner<'a, Ed25519, [u8; 32]> + BoxableDeriveSigner<'a, Secp256k1, [u8; 32]>
// {
// }

struct HasSigner;

impl HasData for HasSigner {
    type Data<'a> = WithAppId<'a, Arc<InMemorySigner>>;
}

impl<'a> Host<'a> {
    #[inline]
    pub fn signer(self) -> WithAppId<'a, Arc<InMemorySigner>> {
        WithAppId::new(self.ctx.app_id, self.signer)
    }
}

pub(crate) fn add_crypto_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: HostView,
{
    bindings::outlayer::crypto::ed25519::add_to_linker::<T, HasSigner>(linker, |t| {
        t.ctx().signer()
    })?;
    bindings::outlayer::crypto::secp256k1::add_to_linker::<T, HasSigner>(linker, |t| {
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
