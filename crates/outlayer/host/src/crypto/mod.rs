mod ed25519;
mod secp256k1;

use std::marker::PhantomData;

use borsh::BorshSerialize;
use defuse_outlayer_crypto::{Curve, signer::InMemorySigner};
use defuse_outlayer_primitives::{AppId, crypto::DerivationPath};
use digest_io::IoWrapper;
use sha3::{Digest, Sha3_256};

use crate::Host;

pub struct CryptoHost<'a> {
    app_id: AppId<'a>,
    signer: &'a InMemorySigner,
}

impl Host<'_> {
    fn tweak(&self, prefix: &[u8], path: impl AsRef<str>) -> [u8; 32] {
        let path = DerivationPath {
            app_id: self.ctx.app_id.as_ref(),
            path: path.as_ref().into(),
        };

        let mut hasher = IoWrapper(Sha3_256::new_with_prefix(prefix));
        borsh::to_writer(&mut hasher, &path).expect("borsh");
        hasher.0.finalize().into()
    }
}

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
