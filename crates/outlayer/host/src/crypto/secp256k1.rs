use defuse_outlayer_crypto::{
    DeriveSigner,
    secp256k1::{Secp256k1, k256::elliptic_curve::sec1::ToEncodedPoint},
};

use crate::Host;

impl crate::bindings::outlayer::crypto::secp256k1::Host for Host {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        let tweak = self.tweak::<Secp256k1>(path);

        let derived_pk = DeriveSigner::<Secp256k1>::derive_public_key(&self.signer, &tweak);

        derived_pk
            .to_encoded_point(false) // uncompressed
            .as_bytes()[1..] // trim leading SEC1 tag byte (0x04)
            .to_vec()
    }

    fn sign(&mut self, path: String, prehash: Vec<u8>) -> Vec<u8> {
        let tweak = self.tweak::<Secp256k1>(path);

        let (signature, recovery_id) =
            DeriveSigner::<Secp256k1>::sign(&self.signer, &tweak, &prehash);

        let mut sig = signature.to_vec();
        sig.push(recovery_id.to_byte());
        sig
    }
}
