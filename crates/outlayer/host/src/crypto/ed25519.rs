use defuse_outlayer_crypto::{
    DeriveSigner, PublicKeyDerivationScheme,
    ed25519::{Ed25519, Ed25519AdditiveDerivation},
};

use crate::Host;

impl crate::bindings::outlayer::crypto::ed25519::Host for Host<'_> {
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        let path = self.tweak(path);

        let derived_pk = DeriveSigner::<Ed25519>::derive_public_key(&self.signer, &path);

        Ok(derived_pk.to_bytes().to_vec())
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        let tweak = self.tweak(path);

        let signature = DeriveSigner::<Ed25519>::derive_sign_from_tweak(&self.signer, &tweak, &msg);

        Ok(signature.to_vec())
    }
}

impl DeriveSigner<Ed25519AdditiveDerivation, str> for Host<'_> {
    #[doc = " Get master public key of the signer"]
    fn public_key(&self) -> <S::Curve as Curve>::PublicKey {
        todo!()
    }

    #[doc = " Sign given message with a secret key **internally** derived for given"]
    #[doc = " [`path`](DerivableCurve::Path)."]
    #[doc = ""]
    #[doc = " NOTE: the returned signatures might be non-deterministic, i.e."]
    #[doc = " implementations MAY return different signatures for the same"]
    #[doc = " `path` and `msg`."]
    fn derive_sign(
        &self,
        path: &P,
        msg: &<S::Curve as Curve>::Message,
    ) -> <S::Curve as Curve>::Signature {
        todo!()
    }
}
