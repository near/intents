#[cfg(feature = "webauthn")]
pub mod webauthn;
// TODO: implmenet other standards

pub trait SigningStandard {
    type PublicKey;

    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &str) -> bool;
}
