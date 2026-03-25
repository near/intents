use crate::signature::SigningStandard;
use defuse_crypto::{Secp256k1PublicKey, SignedPayload};
use defuse_erc191::SignedErc191Payload;
use near_sdk::{serde::de::DeserializeOwned, serde_json};

pub struct Erc191;

impl<M> SigningStandard<&M> for Erc191
where
    M: DeserializeOwned + PartialEq,
{
    type PublicKey = Secp256k1PublicKey;

    fn verify(msg: &M, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(signature) = serde_json::from_str::<SignedErc191Payload>(signature) else {
            return false;
        };

        // deserialize the payload as message
        let Ok(payload) = serde_json::from_str::<M>(&signature.payload.0) else {
            return false;
        };

        // check that signed the same message
        if msg != &payload {
            return false;
        }

        // recover public key from signature
        let Some(recovered_pk) = signature.verify() else {
            return false;
        };

        // check that recovered public key matches contract's one
        recovered_pk == public_key.0
    }
}
