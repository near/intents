use super::{DefusePayload, ExtractDefusePayload};
use defuse_crypto::{Curve, Secp256k1, Secp256k1Signature, SignedPayload};
use defuse_erc191::Erc191;
use defuse_signature_scheme::{
    RecoverableSignatureScheme,
    k256::ecdsa::{RecoveryId, Signature},
};
use near_sdk::{near, serde::de::DeserializeOwned, serde_json};

impl<T> ExtractDefusePayload<T> for SignedErc191Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload)
    }
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SignedErc191Payload {
    pub payload: String,

    /// There is no public key member because the public key can be recovered
    /// via `ecrecover()` knowing the data and the signature
    #[serde_as(as = "defuse_crypto::serde::AsCurve<Secp256k1>")]
    pub signature: Secp256k1Signature,
}

impl defuse_crypto::Payload for SignedErc191Payload {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        Erc191::prehash(&self.payload)
    }
}

impl SignedPayload for SignedErc191Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        let [signature @ .., v] = self.signature.0;
        let signature = Signature::from_bytes(&signature.into()).ok()?;
        let recovery_id = RecoveryId::from_byte(v)?;

        Erc191::recover(&self.payload, self.signature, recovery_id)
    }
}
