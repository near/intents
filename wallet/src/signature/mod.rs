#[cfg(feature = "no-sign")]
pub mod no_sign;
#[cfg(feature = "webauthn")]
pub mod webauthn;

pub trait SigningStandard {
    type PublicKey;

    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &str) -> bool;
}
