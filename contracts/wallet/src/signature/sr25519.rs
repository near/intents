use defuse_crypto::SignedPayload;
pub use defuse_crypto::Sr25519PublicKey;
use defuse_sr25519::SignedSr25519Payload;
use near_sdk::serde_json;

use crate::signature::{RequestMessage, SigningStandard};

/// Sr25519 (Polkadot/Substrate) signing standard for the wallet contract.
///
/// The `proof` submitted to `w_execute_signed` is a JSON
/// [`SignedSr25519Payload`] whose inner `payload` string is the **JSON-encoded
/// [`RequestMessage`]**. Substrate wallets (Polkadot.js, Talisman, …) wrap
/// that string in `<Bytes>...</Bytes>` before signing — verification
/// replicates that wrap inside [`SignedSr25519Payload::verify`].
pub struct Sr25519;

impl SigningStandard<&RequestMessage> for Sr25519 {
    type PublicKey = Sr25519PublicKey;

    fn verify(msg: &RequestMessage, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(signed) = serde_json::from_str::<SignedSr25519Payload>(signature) else {
            return false;
        };

        let Ok(expected) = serde_json::to_string(msg) else {
            return false;
        };
        if signed.payload.payload != expected {
            return false;
        }

        signed.verify() == Some(public_key.0)
    }
}
