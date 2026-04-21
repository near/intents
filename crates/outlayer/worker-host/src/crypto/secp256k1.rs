use defuse_outlayer_host::crypto::secp256k1::{
    Secp256k1Host, Secp256k1PublicKey, Secp256k1Signature,
};
use k256::{
    Secp256k1,
    elliptic_curve::{CurveArithmetic, PrimeField},
};

use crate::WorkerHost;

impl Secp256k1Host for WorkerHost {
    fn secp256k1_derive_public_key(&self, path: &str) -> Secp256k1PublicKey {
        let tweak = [0u8; 32]; // TODO
        let tweak = k256::Scalar::from_repr(tweak.into()).unwrap();

        let pk = (<Secp256k1 as CurveArithmetic>::ProjectivePoint::GENERATOR * tweak
            + self.secp256k1_root_sk.public_key().as_affine())
        .to_affine();

        k256::PublicKey::from_affine(pk)
            .unwrap() // TODO
            .to_sec1_bytes() // TODO
            .to_vec()
            .try_into()
            .unwrap()
    }

    fn secp256k1_sign(&self, _path: &str, _msg: &[u8]) -> Secp256k1Signature {
        unimplemented!()
    }
}
