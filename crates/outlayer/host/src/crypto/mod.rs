mod ed25519;
mod secp256k1;

use defuse_outlayer_crypto::{
    DerivableCurve, DeriveSigner, ed25519::Ed25519, secp256k1::Secp256k1,
};
use defuse_outlayer_primitives::crypto::DerivationPath;

use crate::Host;

pub trait Signer: DeriveSigner<Ed25519> + DeriveSigner<Secp256k1> {}
impl<T> Signer for T where T: DeriveSigner<Ed25519> + DeriveSigner<Secp256k1> {}

impl Host {
    fn tweak<C>(&self, path: impl AsRef<str>) -> C::Tweak
    where
        C: DerivableCurve,
    {
        let path = DerivationPath {
            app_id: self.ctx.app_id.as_ref(),
            path: path.as_ref().into(),
        };

        C::tweak(path.hash())
    }
}
