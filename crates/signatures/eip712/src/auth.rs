//! EIP-712 authorization envelope for NEP-641 `w_resolve_auth`.
//!
//! EIP-712 type:
//! ```text
//! Authorization(string purpose,string recipient,string payload)
//! ```
//!
//! The `accountId` is NOT part of the signed data — the wallet contract
//! knows its own account via `env::current_account_id()`, and the public
//! key is recovered from the ECDSA signature.

use defuse_crypto::{CryptoHash, Curve, Payload, Secp256k1, SignedPayload, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near, serde_with::serde_as};

use crate::{DOMAIN_SEPARATOR, DOMAIN_NAME, DOMAIN_VERSION};

/// `keccak256("Authorization(string purpose,string recipient,string payload)")`
const AUTHORIZATION_TYPEHASH: CryptoHash = [
    0x95, 0xff, 0xbe, 0x89, 0x35, 0x4b, 0x1b, 0xc6,
    0x0a, 0xbd, 0xe5, 0x1f, 0x8f, 0x1e, 0x5c, 0x18,
    0x88, 0x6c, 0x72, 0x82, 0xbc, 0x55, 0x57, 0xb1,
    0x0e, 0x4f, 0x28, 0x45, 0x0c, 0x6f, 0xad, 0x83,
];

pub const AUTHORIZATION_TYPE: &str =
    "Authorization(string purpose,string recipient,string payload)";

/// The EIP-712 message for NEP-641 authorization.
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Eip712Authorization {
    pub purpose: String,
    pub recipient: String,
    pub payload: String,
}

impl Eip712Authorization {
    #[inline]
    fn struct_hash(&self) -> CryptoHash {
        let mut buf = [0u8; 4 * 32]; // typeHash + 3 string fields
        buf[0..32].copy_from_slice(&AUTHORIZATION_TYPEHASH);
        buf[32..64].copy_from_slice(&env::keccak256_array(self.purpose.as_bytes()));
        buf[64..96].copy_from_slice(&env::keccak256_array(self.recipient.as_bytes()));
        buf[96..128].copy_from_slice(&env::keccak256_array(self.payload.as_bytes()));
        env::keccak256_array(buf)
    }
}

impl Payload for Eip712Authorization {
    #[inline]
    fn hash(&self) -> CryptoHash {
        let struct_hash = self.struct_hash();
        let mut buf = [0u8; 66];
        buf[0] = 0x19;
        buf[1] = 0x01;
        buf[2..34].copy_from_slice(&DOMAIN_SEPARATOR);
        buf[34..66].copy_from_slice(&struct_hash);
        env::keccak256_array(buf)
    }
}

/// Signed EIP-712 authorization blob (the `authorization` parameter to `w_resolve_auth`).
#[near(serializers = [json])]
#[autoimpl(Deref using self.message)]
#[derive(Debug, Clone)]
pub struct SignedEip712Authorization {
    #[serde(flatten)]
    pub message: Eip712Authorization,

    #[serde_as(as = "AsCurve<Secp256k1>")]
    pub signature: <Secp256k1 as Curve>::Signature,
}

impl Payload for SignedEip712Authorization {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.message.hash()
    }
}

impl SignedPayload for SignedEip712Authorization {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Secp256k1::verify(&self.signature, &self.message.hash(), &())
    }
}

/// Build the `eth_signTypedData_v4` JSON for an authorization request.
pub fn build_auth_typed_data_json(
    purpose: &str,
    recipient: &str,
    payload: &str,
) -> String {
    let escaped_payload = near_sdk::serde_json::to_string(payload)
        .unwrap_or_else(|_| unreachable!());

    format!(
        concat!(
            r#"{{"types":{{"EIP712Domain":[{{"name":"name","type":"string"}},{{"name":"version","type":"string"}}],"#,
            r#""Authorization":[{{"name":"purpose","type":"string"}},"#,
            r#"{{"name":"recipient","type":"string"}},{{"name":"payload","type":"string"}}]}},"#,
            r#""primaryType":"Authorization","#,
            r#""domain":{{"name":"{domain_name}","version":"{domain_version}"}},"#,
            r#""message":{{"purpose":"{purpose}","recipient":"{recipient}","payload":{payload}}}}}"#,
        ),
        domain_name = DOMAIN_NAME,
        domain_version = DOMAIN_VERSION,
        purpose = purpose,
        recipient = recipient,
        payload = escaped_payload,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_typehash() {
        assert_eq!(
            env::keccak256_array(AUTHORIZATION_TYPE.as_bytes()),
            AUTHORIZATION_TYPEHASH,
        );
    }

    #[test]
    fn test_auth_typed_data_json_is_valid() {
        let td = build_auth_typed_data_json("PROVE_OWNERSHIP", "example.app", "hello");
        let parsed: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&td).unwrap();
        assert_eq!(parsed["primaryType"], "Authorization");
        assert_eq!(parsed["message"]["purpose"], "PROVE_OWNERSHIP");
        assert_eq!(parsed["message"]["recipient"], "example.app");
        assert_eq!(parsed["message"]["payload"], "hello");
        assert!(parsed["message"].get("accountId").is_none());
    }

    #[test]
    fn test_signed_auth_deserializes() {
        let json = r#"{"purpose":"PROVE_OWNERSHIP","recipient":"example.app","payload":"hello","signature":"secp256k1:5wHqR6FiZCotbRfPRskG8RzDRkmFmZy9Wweh4vBtZp8eTJ2TPq6rFY9oc6nZyVmEFEPxoxNwVF9pZd1JN84m7m5me"}"#;
        let signed: SignedEip712Authorization = near_sdk::serde_json::from_str(json).unwrap();
        assert_eq!(signed.message.purpose, "PROVE_OWNERSHIP");
        assert_eq!(signed.message.recipient, "example.app");
    }
}
