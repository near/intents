mod ed25519;
mod secp256k1;

use defuse_outlayer_crypto::DerivableCurve;
use defuse_outlayer_primitives::crypto::DerivationPath;

use crate::State;

impl State<'_> {
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
