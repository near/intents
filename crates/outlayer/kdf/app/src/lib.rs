// #[cfg(feature = "ed25519")]
// mod ed25519;
// #[cfg(feature = "secp256k1")]
// mod secp256k1;

// use std::{borrow::Cow, marker::PhantomData};

// use borsh::BorshSerialize;
// use defuse_outlayer_kdf::{
//     DerivationScheme, DeriveSigner, Identity, SubScheme, crypto::DerivableCurve,
// };
// use defuse_outlayer_primitives::AppId;
// use digest_io::IoWrapper;
// use sha3::{Digest, Sha3_256};

// pub struct AppDerivation<S: ?Sized = Identity>(PhantomData<S>);

// impl<'a, P> AppDerivationPath<'a, P> {
//     fn reduce<S>(self) -> AppDerivationPath<'a>
//     where
//         S: SubScheme<P> + ?Sized,
//         S::Output: Into<Cow<'a, str>>,
//     {
//         AppDerivationPath {
//             app_id: self.app_id,
//             // derive the inner path and convert into string
//             path: S::derive(self.path).into(),
//         }
//     }

//     fn hash(&self, prefix: impl AsRef<[u8]>) -> [u8; 32]
//     where
//         P: BorshSerialize,
//     {
//         let mut hasher = IoWrapper(Sha3_256::new_with_prefix(prefix));
//         borsh::to_writer(&mut hasher, self).expect("borsh");
//         hasher.0.finalize().into()
//     }
// }

// impl AppDerivationPath<'_> {}

// impl<'a, S, P> SubScheme<AppDerivationPath<'a, P>> for AppDerivation<S>
// where
//     S: SubScheme<P> + ?Sized,
//     S::Output: Into<Cow<'a, str>>,
// {
//     type Output = [u8; 32];

//     fn derive(path: AppDerivationPath<'a, P>) -> [u8; 32] {
//         // TODO
//         path.reduce::<S>().hash(b"")
//     }
// }

// impl<'a, C, S, P> DerivationScheme<C, AppDerivationPath<'a, P>> for AppDerivation<S>
// where
//     S: SubScheme<P> + ?Sized,
//     S::Output: Into<Cow<'a, str>>,
//     C: AppDerivableCurveDomain + ?Sized,
// {
//     fn tweak(path: AppDerivationPath<'a, P>) -> C::Tweak {
//         let path = path.reduce::<S>();

//         let hash = path.hash(C::DOMAIN_SEPARATOR);

//         C::tweak(hash)
//     }
// }

// pub trait AppDerivableCurveDomain: DerivableCurve {
//     const DOMAIN_SEPARATOR: &'static [u8];

//     fn tweak(hash: [u8; 32]) -> Self::Tweak;
// }

use borsh::BorshSerialize;
use defuse_outlayer_kdf::SchemaFn;
pub use defuse_outlayer_kdf::{self as kdf, DerivableCurve, DerivationSchema, DeriveSigner};
pub use defuse_outlayer_primitives::AppId;
use digest_io::IoWrapper;
use sha3::{Digest, Sha3_256};

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(BorshSerialize)]
/// **Non-hierarchical** derivation path
pub struct AppDerivationPath<'a> {
    /// Identifier of an application to derive for
    pub app_id: AppId<'a>,
    /// Application-level path
    pub path: &'a str,
}

impl AppDerivationPath<'_> {
    fn hash(&self) -> [u8; 32] {
        let mut hasher = IoWrapper(Sha3_256::new());
        borsh::to_writer(&mut hasher, self).expect("borsh");
        hasher.0.finalize().into()
    }
}

pub struct WithAppId<'a, S> {
    app_id: AppId<'a>,
    next: S,
}

impl<'a, S> WithAppId<'a, S> {
    pub const fn new(app_id: AppId<'a>, next: S) -> Self {
        Self { app_id, next }
    }

    fn wrap(&self, path: impl AsRef<str>) -> [u8; 32] {
        AppDerivationPath {
            app_id: self.app_id.as_ref(),
            path: path.as_ref(),
        }
        .hash()
    }
}

impl<C, P, S> DerivationSchema<C, P> for WithAppId<'_, S>
where
    C: DerivableCurve + ?Sized,
    P: AsRef<str>,
    S: DerivationSchema<C, [u8; 32]>,
{
    type Output = S::Output;

    fn derive(&self, path: P) -> Self::Output {
        self.next.derive(self.wrap(path))
    }
}

impl<C, P, S> DeriveSigner<C, P> for WithAppId<'_, S>
where
    C: DerivableCurve + ?Sized,
    P: AsRef<str>,
    S: DeriveSigner<C, [u8; 32]>,
{
    // type Schema<'a>
    //     = WithAppId<'a, S::Schema<'a>>
    // where
    //     Self: 'a;

    // fn schema(&self) -> Self::Schema<'_> {
    //     WithAppId::new(self.app_id.as_ref(), self.next.schema())
    // }

    fn schema<'a>(&'a self) -> Box<dyn DerivationSchema<C, P, Output = C::Tweak> + 'a>
    where
        C: 'a,
        P: 'a,
    {
        Box::new(WithAppId::new(self.app_id.as_ref(), self.next.schema()))
    }

    fn public_key(&self) -> C::PublicKey {
        self.next.public_key()
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        self.next.derive_sign(self.wrap(path), msg)
    }
}

// pub struct AppSigner<'a, S> {
//     app_id: AppId<'a>,
//     signer: S,
// }

// impl<'a, C, S, P, T> DeriveSigner<C, S, P> for AppSigner<'a, T>
// where
//     C: DerivableCurve + ?Sized,
//     S: DerivationScheme<C, P> + ?Sized,
//     T: DeriveSigner<C, App, AppDerivationPath<'a, P>>,
// {
//     fn public_key(&self) -> C::PublicKey {
//         todo!()
//     }

//     fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
//         let path = AppDerivationPath {
//             app_id: self.app_id.as_ref(),
//             path,
//         };

//         self.signer.derive_sign(path, msg)
//     }
// }
