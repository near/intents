use defuse_crypto::secp256k1::{Secp256k1RecoverableSignature, Secp256k1UncompressedPublicKey};
use defuse_tip191::Tip191;
use near_sdk::{CryptoHash, serde_json};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::payload::{Payload, SignedPayload};

use super::{DefusePayload, ExtractDefusePayload};

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTip191Payload {
    pub payload: String,

    /// There is no public key member because the public key can be recovered
    /// via `ecrecover()` knowing the data and the signature
    pub signature: Secp256k1RecoverableSignature,
}

impl Payload for SignedTip191Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        Tip191::prehash(&self.payload)
    }
}

impl SignedPayload for SignedTip191Payload {
    type PublicKey = Secp256k1UncompressedPublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        let (signature, recovery_id) = self.signature.try_into().ok()?;

        Tip191::recover(&self.payload, &signature, recovery_id).map(Into::into)
    }
}

impl<T> ExtractDefusePayload<T> for SignedTip191Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload)
    }
}
