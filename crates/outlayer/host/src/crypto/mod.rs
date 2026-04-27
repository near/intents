mod ed25519;
mod secp256k1;

use defuse_outlayer_primitives::crypto::DerivationPath;

use crate::State;

impl State<'_> {
    fn tweak(&self, path: impl AsRef<str>) -> [u8; 32] {
        let path = DerivationPath {
            app_id: self.ctx.app_id.as_ref(),
            path: path.as_ref().into(),
        };

        path.hash()
    }
}
