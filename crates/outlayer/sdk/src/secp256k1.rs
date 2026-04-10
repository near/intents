wit_bindgen::generate!({
    path: "../wit",
    world: "secp256k1-world",
});

use crate::{OutlayerError, OutlayerResult};
use outlayer::host::secp256k1;

pub type Secp256k1PublicKey = [u8; 64];
pub type Secp256k1Signature = [u8; 65];

pub fn get_root_public_key() -> OutlayerResult<Secp256k1PublicKey> {
    secp256k1::get_root_public_key()
        .map_err(OutlayerError::FailedToGetPublicKey)?
        .bytes
        .try_into()
        .map_err(|_| OutlayerError::InvalidPublicKeyLength)
}

pub fn derive_public_key(path: impl AsRef<str>) -> OutlayerResult<Secp256k1PublicKey> {
    secp256k1::derive_public_key(path.as_ref())
        .map_err(OutlayerError::FailedToGetPublicKey)?
        .bytes
        .try_into()
        .map_err(|_| OutlayerError::InvalidPublicKeyLength)
}

pub fn sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> OutlayerResult<Secp256k1Signature> {
    secp256k1::sign(path.as_ref(), msg.as_ref())
        .map_err(OutlayerError::FailedToSign)?
        .bytes
        .try_into()
        .map_err(|_| OutlayerError::InvalidSignatureLength)
}
