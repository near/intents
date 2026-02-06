#[cfg(feature = "webauthn")]
pub mod webauthn;

pub trait SigningStandard {
    type PublicKey;

    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &[u8]) -> bool;
}
