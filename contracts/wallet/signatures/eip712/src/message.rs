//! [EIP-712](https://eips.ethereum.org/EIPS/eip-712) typed data signed for
//! wallet contracts.

use core::str::FromStr;

use defuse_crypto::secp256k1::Secp256k1RecoverableSignature;
use defuse_eip712::{Eip712, Hash};
use defuse_wallet::{NearPromise, RequestMessage, Timestamp, WalletOp, offchain::OffchainMessage};
use serde::{Deserialize, Serialize};

use crate::DOMAIN;

/// A typed data structure signed under the wallet-contract
/// [domain](crate::DOMAIN).
pub trait Eip712Message {
    /// `encodeType(typeOf(s))`, i.e. the primary type of this structure.
    ///
    /// Since all wallet-contract messages share a single
    /// [domain](crate::DOMAIN), the primary type is what separates them from
    /// each other.
    const ENCODE_TYPE: &'static str;

    /// `hashStruct(s) = keccak256(typeHash ‖ encodeData(s))`
    fn struct_hash(&self) -> Hash;

    /// `keccak256(0x19 ‖ 0x01 ‖ domainSeparator ‖ hashStruct(s))`, i.e. the
    /// digest an Ethereum wallet signs for this structure.
    #[inline]
    fn prehash(&self) -> Hash {
        Eip712::prehash(&DOMAIN.separator(), &self.struct_hash())
    }
}

/// Signed [EIP-712](https://eips.ethereum.org/EIPS/eip-712) typed data, used
/// as the `proof` of [`WalletEip712`](crate::WalletEip712).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedEip712<M> {
    /// The signed typed data, i.e. the `message` of the
    /// `eth_signTypedData_v4` request.
    pub message: M,

    /// Recoverable 65-byte secp256k1 signature over
    /// [`message.prehash()`](Eip712Message::prehash).
    pub signature: Secp256k1RecoverableSignature,
}

/// Typed data mirroring a [`RequestMessage`], signed for
/// [`w_execute_signed()`](defuse_wallet::contract::Wallet::w_execute_signed).
///
/// ```json
/// {
///   "types": {
///     "EIP712Domain": [
///       { "name": "name", "type": "string" },
///       { "name": "version", "type": "string" }
///     ],
///     "WalletRequest": [
///       { "name": "payForGas", "type": "bool" },
///       { "name": "chainId", "type": "string" },
///       { "name": "signerId", "type": "string" },
///       { "name": "nonce", "type": "uint32" },
///       { "name": "createdAt", "type": "string" },
///       { "name": "timeoutSecs", "type": "uint32" },
///       { "name": "internal", "type": "string" },
///       { "name": "external", "type": "string" }
///     ]
///   },
///   "primaryType": "WalletRequest",
///   "domain": { "name": "NEAR Wallet Contract", "version": "1" },
///   "message": {
///     "payForGas": false,
///     "chainId": "mainnet",
///     "signerId": "0s0000000000000000000000000000000000000000",
///     "nonce": 42,
///     "createdAt": "1970-01-01T00:00:00Z",
///     "timeoutSecs": 3600,
///     "internal": "[{\"op\":\"add_extension\",\"payload\":{\"account_id\":\"extension.near\"}}]",
///     "external": "[]"
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eip712RequestMessage {
    /// [`RequestMessage::pay_for_gas`]
    pub pay_for_gas: bool,

    /// [`RequestMessage::chain_id`]
    pub chain_id: String,

    /// [`RequestMessage::signer_id`]
    pub signer_id: String,

    /// [`RequestMessage::nonce`]
    pub nonce: u32,

    /// [`RequestMessage::created_at`] in RFC-3339 format
    pub created_at: String,

    /// [`RequestMessage::timeout`] in seconds
    pub timeout_secs: u32,

    /// JSON-serialized [`Request::internal`](defuse_wallet::Request::internal),
    /// i.e. the list of [wallet operations](WalletOp) to apply.
    pub internal: String,

    /// JSON-serialized [`Request::external`](defuse_wallet::Request::external),
    /// i.e. the list of [promises](NearPromise) to execute.
    pub external: String,
}

impl Eip712Message for Eip712RequestMessage {
    const ENCODE_TYPE: &'static str = "WalletRequest(bool payForGas,string chainId,string signerId,uint32 nonce,string createdAt,uint32 timeoutSecs,string internal,string external)";

    #[inline]
    fn struct_hash(&self) -> Hash {
        Eip712::hash_struct(
            &Eip712::type_hash(Self::ENCODE_TYPE),
            [
                Eip712::encode_bool(self.pay_for_gas),
                Eip712::encode_bytes(&self.chain_id),
                Eip712::encode_bytes(&self.signer_id),
                Eip712::encode_uint(self.nonce),
                Eip712::encode_bytes(&self.created_at),
                Eip712::encode_uint(self.timeout_secs),
                Eip712::encode_bytes(&self.internal),
                Eip712::encode_bytes(&self.external),
            ],
        )
    }
}

impl Eip712RequestMessage {
    /// Returns whether this typed data denotes exactly given [`RequestMessage`],
    /// i.e. whether the signer authorized this very message.
    ///
    /// Members carrying nested structures ([`internal`](Self::internal) and
    /// [`external`](Self::external)) are compared *semantically*, so their
    /// JSON formatting doesn't have to be canonical.
    #[must_use]
    pub fn matches(&self, msg: &RequestMessage) -> bool {
        self.pay_for_gas == msg.pay_for_gas
            && self.chain_id == msg.chain_id
            && self.signer_id == msg.signer_id.as_str()
            && self.nonce == msg.nonce
            && matches_timestamp(&self.created_at, msg.created_at)
            && matches_timeout(self.timeout_secs, msg.timeout)
            && serde_json::from_str::<Vec<WalletOp>>(&self.internal)
                .is_ok_and(|internal| internal == msg.request.internal)
            && serde_json::from_str::<Vec<NearPromise>>(&self.external)
                .is_ok_and(|external| external == msg.request.external)
    }
}

impl From<&RequestMessage> for Eip712RequestMessage {
    fn from(msg: &RequestMessage) -> Self {
        Self {
            pay_for_gas: msg.pay_for_gas,
            chain_id: msg.chain_id.clone(),
            signer_id: msg.signer_id.to_string(),
            nonce: msg.nonce,
            created_at: msg.created_at.to_string(),
            timeout_secs: msg.timeout.as_secs().try_into().unwrap_or(u32::MAX),
            internal: serde_json::to_string(&msg.request.internal)
                .unwrap_or_else(|_| unreachable!()),
            external: serde_json::to_string(&msg.request.external)
                .unwrap_or_else(|_| unreachable!()),
        }
    }
}

/// Typed data mirroring a NEP-641 [`OffchainMessage`], signed for
/// [`w_resolve_auth()`](defuse_wallet::offchain::AuthResolver::w_resolve_auth).
///
/// ```json
/// {
///   "types": {
///     "EIP712Domain": [
///       { "name": "name", "type": "string" },
///       { "name": "version", "type": "string" }
///     ],
///     "WalletAuth": [
///       { "name": "chainId", "type": "string" },
///       { "name": "signerId", "type": "string" },
///       { "name": "path", "type": "string[]" },
///       { "name": "timestamp", "type": "string" },
///       { "name": "payload", "type": "string" }
///     ]
///   },
///   "primaryType": "WalletAuth",
///   "domain": { "name": "NEAR Wallet Contract", "version": "1" },
///   "message": {
///     "chainId": "mainnet",
///     "signerId": "0s0000000000000000000000000000000000000000",
///     "path": ["master.near"],
///     "timestamp": "1970-01-01T00:00:00Z",
///     "payload": "Login to example.app at 2026-07-16T00:00:00Z"
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eip712AuthMessage {
    /// [`OffchainMessage::chain_id`]
    pub chain_id: String,

    /// [`OffchainMessage::signer_id`]
    pub signer_id: String,

    /// [`OffchainMessage::path`]: bottom-up path to the top-level resolver,
    /// i.e. the direct parent first and the top-level resolver last.
    /// Empty for a top-level authorization.
    pub path: Vec<String>,

    /// [`OffchainMessage::timestamp`] in RFC-3339 format
    pub timestamp: String,

    /// [`OffchainMessage::payload`]
    pub payload: String,
}

impl Eip712Message for Eip712AuthMessage {
    const ENCODE_TYPE: &'static str =
        "WalletAuth(string chainId,string signerId,string[] path,string timestamp,string payload)";

    #[inline]
    fn struct_hash(&self) -> Hash {
        Eip712::hash_struct(
            &Eip712::type_hash(Self::ENCODE_TYPE),
            [
                Eip712::encode_bytes(&self.chain_id),
                Eip712::encode_bytes(&self.signer_id),
                Eip712::encode_array(self.path.iter().map(Eip712::encode_bytes)),
                Eip712::encode_bytes(&self.timestamp),
                Eip712::encode_bytes(&self.payload),
            ],
        )
    }
}

impl Eip712AuthMessage {
    /// Returns whether this typed data denotes exactly given [`OffchainMessage`],
    /// i.e. whether the signer authorized this very message.
    #[must_use]
    pub fn matches(&self, msg: &OffchainMessage) -> bool {
        self.chain_id == msg.chain_id
            && self.signer_id == msg.signer_id.as_str()
            && self.path.len() == msg.path.len()
            && self
                .path
                .iter()
                .zip(&msg.path)
                .all(|(path, expected)| path == expected.as_str())
            && matches_timestamp(&self.timestamp, msg.timestamp)
            && self.payload == msg.payload
    }
}

impl From<&OffchainMessage> for Eip712AuthMessage {
    fn from(msg: &OffchainMessage) -> Self {
        Self {
            chain_id: msg.chain_id.clone(),
            signer_id: msg.signer_id.to_string(),
            path: msg.path.iter().map(ToString::to_string).collect(),
            timestamp: msg.timestamp.to_string(),
            payload: msg.payload.clone(),
        }
    }
}

#[inline]
fn matches_timestamp(s: &str, timestamp: Timestamp) -> bool {
    Timestamp::from_str(s).is_ok_and(|parsed| parsed == timestamp)
}

#[inline]
fn matches_timeout(secs: u32, timeout: core::time::Duration) -> bool {
    u64::from(secs) == timeout.as_secs()
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    /// Known-answer vectors: pin the primary types used on-chain.
    /// Recomputing them differently is a breaking change to the wire format.
    #[rstest]
    #[case(
        Eip712RequestMessage::ENCODE_TYPE,
        hex!("1dd8d3b498246adfecef1936d8120de268cc04e0dc5c6a0fe5a3fffd0597e0c8")
    )]
    #[case(
        Eip712AuthMessage::ENCODE_TYPE,
        hex!("ac4a66d331a12e85c8b62dba6474cc9d52c755fa73733f4e240c8d55275b1ab7")
    )]
    fn type_hash(#[case] encode_type: &str, #[case] expected: Hash) {
        assert_eq!(Eip712::type_hash(encode_type), expected);
    }
}
