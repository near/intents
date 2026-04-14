#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;

pub mod crypto {
    #[cfg(feature = "ed25519")]
    pub use crate::ed25519::ed25519;
    #[cfg(feature = "secp256k1")]
    pub use crate::secp256k1::secp256k1;
}
