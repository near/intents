use crate::signature::SigningStandard;
use defuse_crypto::{Secp256k1PublicKey, SignedPayload};
use defuse_eip712::SignedEip712Payload;
use near_sdk::{serde::de::DeserializeOwned, serde_json};

pub struct Eip712;

impl<M> SigningStandard<&M> for Eip712
where
    M: DeserializeOwned + PartialEq,
{
    type PublicKey = Secp256k1PublicKey;

    fn verify(msg: &M, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(signed) = serde_json::from_str::<SignedEip712Payload>(signature) else {
            return false;
        };

        // deserialize the `msg` field as the expected message type
        let Ok(decoded_msg) = serde_json::from_str::<M>(&signed.payload.msg) else {
            return false;
        };

        // check that the signed message matches the on-chain message
        if msg != &decoded_msg {
            return false;
        };

        // recover public key from the EIP-712 signature
        let Some(recovered_pk) = signed.verify() else {
            return false;
        };

        // check that recovered public key matches contract's one
        recovered_pk == public_key.0
    }
}
