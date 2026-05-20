use crate::signature::{RequestMessage, SigningStandard};
use defuse_crypto::{Secp256k1PublicKey, SignedPayload};
use defuse_eip712::SignedEip712Payload;
use near_sdk::serde_json;

pub struct Eip712;

impl SigningStandard<&RequestMessage> for Eip712 {
    type PublicKey = Secp256k1PublicKey;

    fn verify(msg: &RequestMessage, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(signed) = serde_json::from_str::<SignedEip712Payload>(signature) else {
            return false;
        };

        // Verify each scalar field matches the on-chain RequestMessage
        if signed.payload.chain_id != msg.chain_id {
            return false;
        }
        if signed.payload.signer_id != msg.signer_id.as_str() {
            return false;
        }
        if signed.payload.nonce != msg.nonce {
            return false;
        }
        if signed.payload.timeout_secs != u32::try_from(msg.timeout.as_secs()).unwrap_or(0) {
            return false;
        }

        // Compare created_at by re-serializing the msg's Deadline to ISO-8601
        let msg_created_at = serde_json::to_value(&msg.created_at)
            .ok()
            .and_then(|v| v.as_str().map(String::from));
        let Some(msg_created_at) = msg_created_at else {
            return false;
        };
        if signed.payload.created_at != msg_created_at {
            return false;
        }

        // Verify ops: deserialize the JSON string and compare
        let Ok(decoded_ops) = serde_json::from_str::<Vec<crate::WalletOp>>(&signed.payload.ops)
        else {
            return false;
        };
        if msg.request.ops != decoded_ops {
            return false;
        }

        // Verify out: deserialize the JSON string and compare
        let Ok(decoded_out) = serde_json::from_str::<crate::PromiseDAG>(&signed.payload.out)
        else {
            return false;
        };
        if msg.request.out != decoded_out {
            return false;
        }

        // Recover public key from the EIP-712 signature
        let Some(recovered_pk) = signed.verify() else {
            return false;
        };

        recovered_pk == public_key.0
    }
}
