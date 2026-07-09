pub use defuse_kdf_crypto::p256::{P256CompressedPublicKey, P256Signature};
pub use defuse_webauthn::p256::P256;

use crate::WalletWebauthnAlgorithm;

impl WalletWebauthnAlgorithm for P256 {
    type PublicKey = P256CompressedPublicKey;
    type Signature = P256Signature;
}
