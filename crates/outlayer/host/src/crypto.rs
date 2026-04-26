use defuse_outlayer_crypto::{
    DerivableCurve, DeriveSigner,
    ed25519::Ed25519,
    secp256k1::{Secp256k1, k256::elliptic_curve::sec1::ToEncodedPoint},
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

impl crate::bindings::outlayer::crypto::ed25519::Host for Host {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        let tweak = self.tweak::<Ed25519>(path);

        let derived_pk = DeriveSigner::<Ed25519>::derive_public_key(&self.signer, &tweak);

        derived_pk.to_bytes().to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        let tweak = self.tweak::<Ed25519>(path);

        let signature = crate::crypto::DeriveSigner::<Ed25519>::sign(&self.signer, &tweak, &msg);

        signature.to_vec()
    }
}

impl crate::bindings::outlayer::crypto::secp256k1::Host for Host {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        let tweak = self.tweak::<Secp256k1>(path);

        let derived_pk = DeriveSigner::<Secp256k1>::derive_public_key(&self.signer, &tweak);

        derived_pk
            .to_encoded_point(false) // uncompressed
            .as_bytes()[1..] // trim leading SEC1 tag byte (0x04)
            .to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        let tweak = self.tweak::<Secp256k1>(path);

        let (signature, recovery_id) =
            crate::crypto::DeriveSigner::<Secp256k1>::sign(&self.signer, &tweak, &msg);

        let mut sig = signature.to_vec();
        sig.push(recovery_id.to_byte());
        sig
    }
}
