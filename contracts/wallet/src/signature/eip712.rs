use crate::resolve_auth::{AuthorizationResolution, ErrorKind, Purpose};
use crate::signature::SigningStandard;
use defuse_crypto::{Secp256k1PublicKey, SignedPayload};
use defuse_eip712::auth::SignedEip712Authorization;
use defuse_eip712::SignedEip712Payload;
use near_sdk::{serde::de::DeserializeOwned, serde_json};

use crate::signature::RequestMessage;

pub struct Eip712;

impl SigningStandard<&RequestMessage> for Eip712 {
    type PublicKey = Secp256k1PublicKey;

    fn verify(msg: &RequestMessage, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(signed) = serde_json::from_str::<SignedEip712Payload>(signature) else {
            return false;
        };

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

        let msg_created_at = serde_json::to_value(&msg.created_at)
            .ok()
            .and_then(|v| v.as_str().map(String::from));
        let Some(msg_created_at) = msg_created_at else {
            return false;
        };
        if signed.payload.created_at != msg_created_at {
            return false;
        }

        let Ok(decoded_ops) = serde_json::from_str::<Vec<crate::WalletOp>>(&signed.payload.ops)
        else {
            return false;
        };
        if msg.request.ops != decoded_ops {
            return false;
        }

        let Ok(decoded_out) = serde_json::from_str::<crate::PromiseDAG>(&signed.payload.out)
        else {
            return false;
        };
        if msg.request.out != decoded_out {
            return false;
        }

        let Some(recovered_pk) = signed.verify() else {
            return false;
        };

        recovered_pk == public_key.0
    }

    fn resolve_auth(
        purpose: &Purpose,
        recipient: &str,
        authorization: &str,
        public_key: &Self::PublicKey,
    ) -> AuthorizationResolution {
        let signed: SignedEip712Authorization = match serde_json::from_str(authorization) {
            Ok(s) => s,
            Err(e) => {
                return AuthorizationResolution::Invalid {
                    error_kind: ErrorKind::InvalidInput,
                    error_message: format!("failed to decode authorization: {e}"),
                }
            }
        };

        if signed.message.purpose != purpose.to_string() {
            return AuthorizationResolution::Invalid {
                error_kind: ErrorKind::InvalidInput,
                error_message: "purpose mismatch".into(),
            };
        }

        if signed.message.recipient != recipient {
            return AuthorizationResolution::Invalid {
                error_kind: ErrorKind::InvalidInput,
                error_message: "recipient mismatch".into(),
            };
        }

        let Some(recovered_pk) = signed.verify() else {
            return AuthorizationResolution::Invalid {
                error_kind: ErrorKind::InvalidSignature,
                error_message: "signature recovery failed".into(),
            };
        };

        if recovered_pk != public_key.0 {
            return AuthorizationResolution::Invalid {
                error_kind: ErrorKind::InvalidSignature,
                error_message: "recovered public key does not match wallet".into(),
            };
        }

        AuthorizationResolution::Resolved {
            payload: signed.message.payload,
        }
    }
}
