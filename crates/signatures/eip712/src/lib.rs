use defuse_crypto::{CryptoHash, Curve, Payload, Secp256k1, SignedPayload, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near, serde_with::serde_as};

// ─── EIP-712 Domain Constants ────────────────────────────────────────────────

/// Domain name used in `EIP712Domain`.
pub const DOMAIN_NAME: &str = "NEAR Wallet Contract";

/// Domain version used in `EIP712Domain`.
pub const DOMAIN_VERSION: &str = "1";

/// The full EIP-712 domain type string.
pub const EIP712_DOMAIN_TYPE: &str = "EIP712Domain(string name,string version)";

/// The EIP-712 primary type string.
pub const WALLET_MESSAGE_TYPE: &str = "WalletMessage(string msg)";

/// Pre-computed domain separator: `keccak256(abi.encode(typeHash, nameHash, versionHash))`
///
/// where:
///   - `typeHash  = keccak256("EIP712Domain(string name,string version)")`
///   - `nameHash  = keccak256("NEAR Wallet Contract")`
///   - `versionHash = keccak256("1")`
///
/// See [`test_domain_separator`] for the derivation.
const DOMAIN_SEPARATOR: CryptoHash = [
    0x9e, 0xd0, 0x38, 0x4a, 0x93, 0x69, 0xf4, 0x2a,
    0xa8, 0x9e, 0x17, 0x2b, 0x67, 0x35, 0x9e, 0x73,
    0x23, 0x8f, 0xce, 0x30, 0x3b, 0x45, 0xb9, 0x28,
    0x42, 0xe4, 0x71, 0x87, 0x4e, 0x2f, 0x6d, 0x22,
];

/// Pre-computed `keccak256("WalletMessage(string msg)")`.
///
/// See [`test_wallet_message_typehash`] for the derivation.
const WALLET_MESSAGE_TYPEHASH: CryptoHash = [
    0x21, 0xb8, 0x00, 0xf9, 0xa0, 0x01, 0x86, 0x83,
    0x3a, 0x32, 0xc2, 0xc1, 0xdb, 0x0a, 0x39, 0xea,
    0xf1, 0x6e, 0x0e, 0x5a, 0x02, 0xcd, 0x2a, 0x6a,
    0x31, 0xc0, 0x42, 0xe7, 0x69, 0x40, 0xff, 0xfb,
];

// ─── EIP-712 Typed Data Structure ────────────────────────────────────────────
//
// Clients MUST use the following JSON with `eth_signTypedData_v4`:
//
// ```json
// {
//   "types": {
//     "EIP712Domain": [
//       { "name": "name",    "type": "string" },
//       { "name": "version", "type": "string" }
//     ],
//     "WalletMessage": [
//       { "name": "msg", "type": "string" }
//     ]
//   },
//   "primaryType": "WalletMessage",
//   "domain": {
//     "name": "NEAR Wallet Contract",
//     "version": "1"
//   },
//   "message": {
//     "msg": "<JSON-serialized RequestMessage>"
//   }
// }
// ```
//
// The `proof` string submitted to `w_execute_signed` is a JSON object:
//
// ```json
// {
//   "msg": "<JSON-serialized RequestMessage>",
//   "signature": "secp256k1:<base58-encoded 65-byte signature>"
// }
// ```

/// The EIP-712 message structure: `WalletMessage { msg }`.
///
/// `msg` contains the JSON-serialized `RequestMessage` — the same string
/// the signer passes as the `msg` field to `eth_signTypedData_v4`.
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Eip712Payload {
    /// JSON-serialized wallet-contract `RequestMessage`.
    pub msg: String,
}

impl Eip712Payload {
    /// Compute `hashStruct(WalletMessage)` per EIP-712 §3.
    ///
    /// ```text
    /// hashStruct(s) = keccak256(typeHash ‖ encodeData(s))
    /// encodeData(WalletMessage { msg }) = keccak256(bytes(msg))
    /// ```
    #[inline]
    fn struct_hash(&self) -> CryptoHash {
        let msg_hash = env::keccak256_array(self.msg.as_bytes());

        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&WALLET_MESSAGE_TYPEHASH);
        buf[32..].copy_from_slice(&msg_hash);

        env::keccak256_array(buf)
    }
}

impl Payload for Eip712Payload {
    /// Compute the EIP-712 signing hash:
    ///
    /// ```text
    /// keccak256("\x19\x01" ‖ domainSeparator ‖ hashStruct(message))
    /// ```
    #[inline]
    fn hash(&self) -> CryptoHash {
        let struct_hash = self.struct_hash();

        let mut buf = [0u8; 66]; // 2 + 32 + 32
        buf[0] = 0x19;
        buf[1] = 0x01;
        buf[2..34].copy_from_slice(&DOMAIN_SEPARATOR);
        buf[34..66].copy_from_slice(&struct_hash);

        env::keccak256_array(buf)
    }
}

/// Signed EIP-712 payload submitted as the `proof` to `w_execute_signed`.
///
/// JSON format:
/// ```json
/// {
///   "msg": "<JSON-serialized RequestMessage>",
///   "signature": "secp256k1:<base58>"
/// }
/// ```
#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedEip712Payload {
    #[serde(flatten)]
    pub payload: Eip712Payload,

    #[serde_as(as = "AsCurve<Secp256k1>")]
    pub signature: <Secp256k1 as Curve>::Signature,
}

impl Payload for SignedEip712Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedEip712Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Secp256k1::verify(&self.signature, &self.payload.hash(), &())
    }
}

// ─── Client helpers ──────────────────────────────────────────────────────────

/// Build the complete `eth_signTypedData_v4` parameter JSON for a given
/// `RequestMessage` JSON string.
///
/// Returns a JSON string ready to be passed to an Ethereum wallet via
/// WalletConnect or an injected provider.
///
/// ```text
/// { "types": { ... }, "primaryType": "WalletMessage", "domain": { ... }, "message": { "msg": "..." } }
/// ```
pub fn build_typed_data_json(request_message_json: &str) -> String {
    // Escape the JSON string for embedding inside another JSON value.
    // serde_json::to_string serializes a &str as a JSON string with proper escaping.
    let escaped_msg = near_sdk::serde_json::to_string(request_message_json)
        .unwrap_or_else(|_| unreachable!());

    format!(
        r#"{{"types":{{"EIP712Domain":[{{"name":"name","type":"string"}},{{"name":"version","type":"string"}}],"WalletMessage":[{{"name":"msg","type":"string"}}]}},"primaryType":"WalletMessage","domain":{{"name":"{DOMAIN_NAME}","version":"{DOMAIN_VERSION}"}},"message":{{"msg":{escaped_msg}}}}}"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    /// Verify the pre-computed DOMAIN_SEPARATOR constant.
    #[test]
    fn test_domain_separator() {
        let type_hash = env::keccak256_array(EIP712_DOMAIN_TYPE.as_bytes());
        let name_hash = env::keccak256_array(DOMAIN_NAME.as_bytes());
        let version_hash = env::keccak256_array(DOMAIN_VERSION.as_bytes());

        let mut encoded = [0u8; 96];
        encoded[..32].copy_from_slice(&type_hash);
        encoded[32..64].copy_from_slice(&name_hash);
        encoded[64..96].copy_from_slice(&version_hash);

        let computed = env::keccak256_array(encoded);
        assert_eq!(computed, DOMAIN_SEPARATOR);
    }

    /// Verify the pre-computed WALLET_MESSAGE_TYPEHASH constant.
    #[test]
    fn test_wallet_message_typehash() {
        let computed = env::keccak256_array(WALLET_MESSAGE_TYPE.as_bytes());
        assert_eq!(computed, WALLET_MESSAGE_TYPEHASH);
    }

    #[test]
    fn test_eip712_hash_is_deterministic() {
        let p1 = Eip712Payload { msg: "Hello world!".into() };
        let p2 = Eip712Payload { msg: "Hello world!".into() };
        assert_eq!(p1.hash(), p2.hash());

        let p3 = Eip712Payload { msg: "Different".into() };
        assert_ne!(p1.hash(), p3.hash());
    }

    #[test]
    fn test_eip712_struct_hash_follows_spec() {
        let payload = Eip712Payload { msg: "test".into() };
        let sh = payload.struct_hash();

        let type_hash = env::keccak256_array(WALLET_MESSAGE_TYPE.as_bytes());
        let msg_hash = env::keccak256_array(b"test");
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&type_hash);
        buf[32..].copy_from_slice(&msg_hash);
        assert_eq!(sh, env::keccak256_array(buf));
    }

    #[test]
    fn test_typed_data_json_is_valid() {
        let msg = r#"{"chain_id":"mainnet","signer_id":"alice.near"}"#;
        let td = build_typed_data_json(msg);
        let parsed: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&td).unwrap();

        assert_eq!(parsed["primaryType"], "WalletMessage");
        assert_eq!(parsed["domain"]["name"], DOMAIN_NAME);
        assert_eq!(parsed["domain"]["version"], DOMAIN_VERSION);
        assert_eq!(parsed["message"]["msg"], msg);

        // types
        let types = &parsed["types"];
        assert!(types["EIP712Domain"].is_array());
        assert!(types["WalletMessage"].is_array());
        assert_eq!(types["WalletMessage"][0]["name"], "msg");
        assert_eq!(types["WalletMessage"][0]["type"], "string");
    }

    #[test]
    fn test_signed_payload_deserializes() {
        let json = r#"{"msg":"hello","signature":"secp256k1:5wHqR6FiZCotbRfPRskG8RzDRkmFmZy9Wweh4vBtZp8eTJ2TPq6rFY9oc6nZyVmEFEPxoxNwVF9pZd1JN84m7m5me"}"#;
        let signed: SignedEip712Payload = near_sdk::serde_json::from_str(json).unwrap();
        assert_eq!(signed.payload.msg, "hello");
    }

    // ── End-to-end signature verification ────────────────────────────

    // Signature produced offline with private key:
    //   a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56
    // over the EIP-712 typed data for WalletMessage { msg: "Hello world!" }
    const REFERENCE_PUBKEY: [u8; 64] = hex!(
        "85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b8"
        "01f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"
    );
    const REFERENCE_SIGNATURE: [u8; 65] = hex!(
        "09ee2db3a52ba7c36789fc86cde18ba9ba35e206d1a79aca305f3137d81ceeea"
        "37b8239c0f3c884ba1ab9eaff9e259d50404c99b3d7d3145ef2a39250148d4eb"
        "01"
    );
    const REFERENCE_MESSAGE: &str = "Hello world!";

    #[test]
    fn test_reference_signature_verification_works() {
        assert_eq!(
            SignedEip712Payload {
                payload: Eip712Payload { msg: REFERENCE_MESSAGE.into() },
                signature: REFERENCE_SIGNATURE,
            }
            .verify(),
            Some(REFERENCE_PUBKEY),
        );
    }

    #[test]
    fn test_wrong_message_verification_fails() {
        assert_ne!(
            SignedEip712Payload {
                payload: Eip712Payload { msg: "Wrong message".into() },
                signature: REFERENCE_SIGNATURE,
            }
            .verify(),
            Some(REFERENCE_PUBKEY),
        );
    }

    #[test]
    fn test_corrupted_signature_verification_fails() {
        let mut bad_sig = REFERENCE_SIGNATURE;
        bad_sig[0] ^= 0xff;
        assert_ne!(
            SignedEip712Payload {
                payload: Eip712Payload { msg: REFERENCE_MESSAGE.into() },
                signature: bad_sig,
            }
            .verify(),
            Some(REFERENCE_PUBKEY),
        );
    }
}
