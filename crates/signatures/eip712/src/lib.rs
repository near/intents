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
pub const WALLET_MESSAGE_TYPE: &str =
    "WalletMessage(string chainId,string signerId,uint32 nonce,string createdAt,uint32 timeoutSecs,string ops,string out)";

/// Pre-computed domain separator.
/// See [`test_domain_separator`] for the derivation.
const DOMAIN_SEPARATOR: CryptoHash = [
    0x9e, 0xd0, 0x38, 0x4a, 0x93, 0x69, 0xf4, 0x2a,
    0xa8, 0x9e, 0x17, 0x2b, 0x67, 0x35, 0x9e, 0x73,
    0x23, 0x8f, 0xce, 0x30, 0x3b, 0x45, 0xb9, 0x28,
    0x42, 0xe4, 0x71, 0x87, 0x4e, 0x2f, 0x6d, 0x22,
];

/// Pre-computed `keccak256(WALLET_MESSAGE_TYPE)`.
/// See [`test_wallet_message_typehash`] for the derivation.
const WALLET_MESSAGE_TYPEHASH: CryptoHash = [
    0x58, 0xbf, 0x0b, 0x90, 0xf2, 0x8e, 0xcd, 0x57,
    0xe8, 0x5c, 0x83, 0x0e, 0x6e, 0x40, 0x7f, 0x9b,
    0xbf, 0x49, 0x3c, 0x27, 0xf8, 0x16, 0x44, 0x23,
    0x98, 0xce, 0xfe, 0x24, 0xc1, 0x86, 0x5e, 0xc5,
];

// ─── EIP-712 Typed Data Structure ────────────────────────────────────────────
//
// Clients MUST use the following JSON with `eth_signTypedData_v4`:
//
// ```json
// {
//   "types": {
//     "EIP712Domain": [
//       { "name": "name",        "type": "string" },
//       { "name": "version",     "type": "string" }
//     ],
//     "WalletMessage": [
//       { "name": "chainId",     "type": "string" },
//       { "name": "signerId",    "type": "string" },
//       { "name": "nonce",       "type": "uint32" },
//       { "name": "createdAt",   "type": "string" },
//       { "name": "timeoutSecs", "type": "uint32" },
//       { "name": "ops",         "type": "string" },
//       { "name": "out",         "type": "string" }
//     ]
//   },
//   "primaryType": "WalletMessage",
//   "domain": {
//     "name": "NEAR Wallet Contract",
//     "version": "1"
//   },
//   "message": {
//     "chainId":     "mainnet",
//     "signerId":    "0s...",
//     "nonce":       12345,
//     "createdAt":   "2026-05-07T00:00:00Z",
//     "timeoutSecs": 3600,
//     "ops":         "[]",
//     "out":         "{\"after\":[],\"then\":[...]}"
//   }
// }
// ```

/// The EIP-712 message structure with individual wallet-contract fields.
#[near(serializers = [json])]
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Eip712Payload {
    pub chain_id: String,
    pub signer_id: String,
    pub nonce: u32,
    pub created_at: String,
    pub timeout_secs: u32,
    /// JSON-serialized wallet-contract `ops` (wallet operations).
    pub ops: String,
    /// JSON-serialized wallet-contract `out` (promise DAG).
    pub out: String,
}

impl Eip712Payload {
    fn encode_uint32(value: u32) -> [u8; 32] {
        let mut buf = [0u8; 32];
        buf[28..32].copy_from_slice(&value.to_be_bytes());
        buf
    }

    /// Compute `hashStruct(WalletMessage)` per EIP-712 §3.
    ///
    /// For each field:
    ///   - `string`:  `keccak256(bytes(value))`
    ///   - `uint32`:  `abi.encode(value)` (left-padded to 32 bytes)
    #[inline]
    fn struct_hash(&self) -> CryptoHash {
        // 8 words: typeHash + 7 fields
        let mut buf = [0u8; 8 * 32];
        buf[0..32].copy_from_slice(&WALLET_MESSAGE_TYPEHASH);
        buf[32..64].copy_from_slice(&env::keccak256_array(self.chain_id.as_bytes()));
        buf[64..96].copy_from_slice(&env::keccak256_array(self.signer_id.as_bytes()));
        buf[96..128].copy_from_slice(&Self::encode_uint32(self.nonce));
        buf[128..160].copy_from_slice(&env::keccak256_array(self.created_at.as_bytes()));
        buf[160..192].copy_from_slice(&Self::encode_uint32(self.timeout_secs));
        buf[192..224].copy_from_slice(&env::keccak256_array(self.ops.as_bytes()));
        buf[224..256].copy_from_slice(&env::keccak256_array(self.out.as_bytes()));

        env::keccak256_array(buf)
    }
}

impl Payload for Eip712Payload {
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

/// Signed EIP-712 payload submitted as the `proof` to `w_execute_signed`.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> Eip712Payload {
        Eip712Payload {
            chain_id: "mainnet".into(),
            signer_id: "test.near".into(),
            nonce: 42,
            created_at: "2026-01-01T00:00:00Z".into(),
            timeout_secs: 3600,
            ops: "[]".into(),
            out: r#"{"after":[],"then":[]}"#.into(),
        }
    }

    #[test]
    fn test_domain_separator() {
        let type_hash = env::keccak256_array(EIP712_DOMAIN_TYPE.as_bytes());
        let name_hash = env::keccak256_array(DOMAIN_NAME.as_bytes());
        let version_hash = env::keccak256_array(DOMAIN_VERSION.as_bytes());

        let mut encoded = [0u8; 96];
        encoded[..32].copy_from_slice(&type_hash);
        encoded[32..64].copy_from_slice(&name_hash);
        encoded[64..96].copy_from_slice(&version_hash);

        assert_eq!(env::keccak256_array(encoded), DOMAIN_SEPARATOR);
    }

    #[test]
    fn test_wallet_message_typehash() {
        assert_eq!(
            env::keccak256_array(WALLET_MESSAGE_TYPE.as_bytes()),
            WALLET_MESSAGE_TYPEHASH,
        );
    }

    #[test]
    fn test_eip712_hash_is_deterministic() {
        let p1 = sample_payload();
        let p2 = sample_payload();
        assert_eq!(p1.hash(), p2.hash());

        let mut p3 = sample_payload();
        p3.out = r#"{"after":[],"then":[{"receiver_id":"x"}]}"#.into();
        assert_ne!(p1.hash(), p3.hash());
    }

    #[test]
    fn test_struct_hash_follows_eip712_spec() {
        let p = sample_payload();
        let sh = p.struct_hash();

        let mut buf = [0u8; 256];
        buf[0..32].copy_from_slice(&env::keccak256_array(WALLET_MESSAGE_TYPE.as_bytes()));
        buf[32..64].copy_from_slice(&env::keccak256_array(b"mainnet"));
        buf[64..96].copy_from_slice(&env::keccak256_array(b"test.near"));
        buf[96..128].copy_from_slice(&Eip712Payload::encode_uint32(42));
        buf[128..160].copy_from_slice(&env::keccak256_array(b"2026-01-01T00:00:00Z"));
        buf[160..192].copy_from_slice(&Eip712Payload::encode_uint32(3600));
        buf[192..224].copy_from_slice(&env::keccak256_array(b"[]"));
        buf[224..256].copy_from_slice(&env::keccak256_array(br#"{"after":[],"then":[]}"#));

        assert_eq!(sh, env::keccak256_array(buf));
    }

    #[test]
    fn test_signed_payload_deserializes() {
        let json = r#"{"chainId":"mainnet","signerId":"test.near","nonce":1,"createdAt":"2026-01-01T00:00:00Z","timeoutSecs":3600,"ops":"[]","out":"{}","signature":"secp256k1:5wHqR6FiZCotbRfPRskG8RzDRkmFmZy9Wweh4vBtZp8eTJ2TPq6rFY9oc6nZyVmEFEPxoxNwVF9pZd1JN84m7m5me"}"#;
        let signed: SignedEip712Payload = near_sdk::serde_json::from_str(json).unwrap();
        assert_eq!(signed.payload.chain_id, "mainnet");
        assert_eq!(signed.payload.ops, "[]");
        assert_eq!(signed.payload.out, "{}");
    }
}
