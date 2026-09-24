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
    /// `encodeType(typeOf(s))`, i.e. the primary type of this structure
    /// followed by the definitions of all referenced struct types, sorted
    /// alphabetically.
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
/// The request contents are *unpacked* into typed structures
/// ([`Eip712WalletOp`], [`Eip712NearPromise`], [`Eip712NearAction`]), so the
/// Ethereum wallet renders each operation and promise structurally instead of
/// one opaque JSON blob. Only the leaf `payload`s remain JSON.
///
/// ```json
/// {
///   "types": {
///     "EIP712Domain": [
///       { "name": "name", "type": "string" },
///       { "name": "version", "type": "string" }
///     ],
///     "WalletRequest": [
///       { "name": "chainId", "type": "string" },
///       { "name": "signerId", "type": "string" },
///       { "name": "nonce", "type": "uint32" },
///       { "name": "createdAt", "type": "string" },
///       { "name": "timeoutSecs", "type": "uint32" },
///       { "name": "internal", "type": "WalletOp[]" },
///       { "name": "external", "type": "NearPromise[]" },
///       { "name": "payForGas", "type": "bool" }
///     ],
///     "WalletOp": [
///       { "name": "op", "type": "string" },
///       { "name": "payload", "type": "string" }
///     ],
///     "NearPromise": [
///       { "name": "receiverId", "type": "string" },
///       { "name": "refundTo", "type": "string" },
///       { "name": "actions", "type": "NearAction[]" }
///     ],
///     "NearAction": [
///       { "name": "action", "type": "string" },
///       { "name": "payload", "type": "string" }
///     ]
///   },
///   "primaryType": "WalletRequest",
///   "domain": { "name": "NEAR Wallet Contract", "version": "1" },
///   "message": {
///     "chainId": "mainnet",
///     "signerId": "0s0000000000000000000000000000000000000000",
///     "nonce": 42,
///     "createdAt": "1970-01-01T00:00:00Z",
///     "timeoutSecs": 3600,
///     "internal": [
///       { "op": "add_extension", "payload": "{\"account_id\":\"extension.near\"}" }
///     ],
///     "external": [
///       {
///         "receiverId": "bob.near",
///         "refundTo": "",
///         "actions": [
///           { "action": "transfer", "payload": "{\"deposit\":\"1000000000000000000000000\"}" }
///         ]
///       }
///     ],
///     "payForGas": false
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eip712RequestMessage {
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

    /// [`Request::internal`](defuse_wallet::Request::internal), i.e. the list
    /// of [wallet operations](WalletOp) to apply.
    pub internal: Vec<Eip712WalletOp>,

    /// [`Request::external`](defuse_wallet::Request::external), i.e. the list
    /// of [promises](NearPromise) to execute.
    pub external: Vec<Eip712NearPromise>,

    /// [`RequestMessage::pay_for_gas`]
    ///
    /// The member is always part of the signed type (EIP-712 has no optional
    /// members), but it MAY be omitted from the JSON representation, in which
    /// case it defaults to `false`.
    #[serde(default)]
    pub pay_for_gas: bool,
}

impl Eip712Message for Eip712RequestMessage {
    const ENCODE_TYPE: &'static str = "WalletRequest(string chainId,string signerId,uint32 nonce,string createdAt,uint32 timeoutSecs,WalletOp[] internal,NearPromise[] external,bool payForGas)NearAction(string action,string payload)NearPromise(string receiverId,string refundTo,NearAction[] actions)WalletOp(string op,string payload)";

    #[inline]
    fn struct_hash(&self) -> Hash {
        Eip712::hash_struct(
            &Eip712::type_hash(Self::ENCODE_TYPE),
            [
                Eip712::encode_bytes(&self.chain_id),
                Eip712::encode_bytes(&self.signer_id),
                Eip712::encode_uint(self.nonce),
                Eip712::encode_bytes(&self.created_at),
                Eip712::encode_uint(self.timeout_secs),
                Eip712::encode_array(self.internal.iter().map(Eip712WalletOp::struct_hash)),
                Eip712::encode_array(self.external.iter().map(Eip712NearPromise::struct_hash)),
                Eip712::encode_bool(self.pay_for_gas),
            ],
        )
    }
}

impl Eip712RequestMessage {
    /// Returns whether this typed data denotes exactly given [`RequestMessage`],
    /// i.e. whether the signer authorized this very message.
    ///
    /// Leaf `payload`s are compared *semantically* (see
    /// [`Eip712WalletOp::matches()`]), so their JSON formatting doesn't have
    /// to be canonical.
    #[must_use]
    pub fn matches(&self, msg: &RequestMessage) -> bool {
        self.chain_id == msg.chain_id
            && self.signer_id == msg.signer_id.as_str()
            && self.nonce == msg.nonce
            && matches_timestamp(&self.created_at, msg.created_at)
            && matches_timeout(self.timeout_secs, msg.timeout)
            && self.internal.len() == msg.request.internal.len()
            && self
                .internal
                .iter()
                .zip(&msg.request.internal)
                .all(|(op, expected)| op.matches(expected))
            && self.external.len() == msg.request.external.len()
            && self
                .external
                .iter()
                .zip(&msg.request.external)
                .all(|(promise, expected)| promise.matches(expected))
            && self.pay_for_gas == msg.pay_for_gas
    }
}

impl From<&RequestMessage> for Eip712RequestMessage {
    fn from(msg: &RequestMessage) -> Self {
        Self {
            chain_id: msg.chain_id.clone(),
            signer_id: msg.signer_id.to_string(),
            nonce: msg.nonce,
            created_at: msg.created_at.to_string(),
            timeout_secs: msg.timeout.as_secs().try_into().unwrap_or(u32::MAX),
            internal: msg.request.internal.iter().map(Into::into).collect(),
            external: msg.request.external.iter().map(Into::into).collect(),
            pay_for_gas: msg.pay_for_gas,
        }
    }
}

/// Typed data mirroring a [`WalletOp`]: the operation tag along with its
/// JSON-serialized payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Eip712WalletOp {
    /// The operation tag, e.g. `add_extension`.
    pub op: String,

    /// JSON-serialized operation payload, e.g.
    /// `{"account_id":"extension.near"}`.
    pub payload: String,
}

impl Eip712WalletOp {
    /// `encodeType(WalletOp)`
    pub const ENCODE_TYPE: &'static str = "WalletOp(string op,string payload)";

    /// `hashStruct(s) = keccak256(typeHash ‖ encodeData(s))`
    #[inline]
    pub fn struct_hash(&self) -> Hash {
        Eip712::hash_struct(
            &Eip712::type_hash(Self::ENCODE_TYPE),
            [
                Eip712::encode_bytes(&self.op),
                Eip712::encode_bytes(&self.payload),
            ],
        )
    }

    /// Returns whether this typed data denotes exactly given [`WalletOp`].
    ///
    /// The [`payload`](Self::payload) is compared *semantically* against the
    /// canonical serialization of `expected`: JSON whitespace and key order
    /// are free, but the set of fields must match exactly — extra fields
    /// (which the contract would otherwise silently ignore) invalidate the
    /// proof, so the wallet can never display more than what gets executed.
    #[must_use]
    pub fn matches(&self, expected: &WalletOp) -> bool {
        let expected = Self::from(expected);
        self.op == expected.op && matches_json(&self.payload, &expected.payload)
    }
}

impl From<&WalletOp> for Eip712WalletOp {
    fn from(op: &WalletOp) -> Self {
        let (op, payload) = split_tagged(op, "op");
        Self { op, payload }
    }
}

/// Typed data mirroring a [`NearPromise`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eip712NearPromise {
    /// [`NearPromise::receiver_id`]
    pub receiver_id: String,

    /// [`NearPromise::refund_to`], or an empty string if not set.
    pub refund_to: String,

    /// [`NearPromise::actions`]
    pub actions: Vec<Eip712NearAction>,
}

impl Eip712NearPromise {
    /// `encodeType(NearPromise)`, including the referenced
    /// [`NearAction`](Eip712NearAction) type.
    pub const ENCODE_TYPE: &'static str = "NearPromise(string receiverId,string refundTo,NearAction[] actions)NearAction(string action,string payload)";

    /// `hashStruct(s) = keccak256(typeHash ‖ encodeData(s))`
    #[inline]
    pub fn struct_hash(&self) -> Hash {
        Eip712::hash_struct(
            &Eip712::type_hash(Self::ENCODE_TYPE),
            [
                Eip712::encode_bytes(&self.receiver_id),
                Eip712::encode_bytes(&self.refund_to),
                Eip712::encode_array(self.actions.iter().map(Eip712NearAction::struct_hash)),
            ],
        )
    }

    /// Returns whether this typed data denotes exactly given [`NearPromise`].
    #[must_use]
    pub fn matches(&self, expected: &NearPromise) -> bool {
        self.receiver_id == expected.receiver_id.as_str()
            && expected
                .refund_to
                .as_ref()
                .map_or(self.refund_to.is_empty(), |refund_to| {
                    self.refund_to == refund_to.as_str()
                })
            && self.actions.len() == expected.actions.len()
            && self
                .actions
                .iter()
                .zip(&expected.actions)
                .all(|(action, expected)| action.matches(expected))
    }
}

impl From<&NearPromise> for Eip712NearPromise {
    fn from(promise: &NearPromise) -> Self {
        Self {
            receiver_id: promise.receiver_id.to_string(),
            refund_to: promise
                .refund_to
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            actions: promise.actions.iter().map(Into::into).collect(),
        }
    }
}

/// Typed data mirroring a [`NearAction`](defuse_wallet::actions::NearAction):
/// the action tag along with its JSON-serialized payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Eip712NearAction {
    /// The action tag, e.g. `function_call` or `transfer`.
    pub action: String,

    /// JSON-serialized action payload, e.g. `{"deposit":"1"}`.
    pub payload: String,
}

impl Eip712NearAction {
    /// `encodeType(NearAction)`
    pub const ENCODE_TYPE: &'static str = "NearAction(string action,string payload)";

    /// `hashStruct(s) = keccak256(typeHash ‖ encodeData(s))`
    #[inline]
    pub fn struct_hash(&self) -> Hash {
        Eip712::hash_struct(
            &Eip712::type_hash(Self::ENCODE_TYPE),
            [
                Eip712::encode_bytes(&self.action),
                Eip712::encode_bytes(&self.payload),
            ],
        )
    }

    /// Returns whether this typed data denotes exactly given
    /// [`NearAction`](defuse_wallet::actions::NearAction).
    ///
    /// The [`payload`](Self::payload) is compared *semantically*, see
    /// [`Eip712WalletOp::matches()`].
    #[must_use]
    pub fn matches(&self, expected: &defuse_wallet::actions::NearAction) -> bool {
        let expected = Self::from(expected);
        self.action == expected.action && matches_json(&self.payload, &expected.payload)
    }
}

impl From<&defuse_wallet::actions::NearAction> for Eip712NearAction {
    fn from(action: &defuse_wallet::actions::NearAction) -> Self {
        let (action, payload) = split_tagged(action, "action");
        Self { action, payload }
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

/// Split an adjacently-tagged enum (`{"<tag>": ..., "payload": ...}`) into
/// its tag and JSON-serialized payload.
fn split_tagged<T: Serialize>(value: &T, tag: &str) -> (String, String) {
    let Ok(serde_json::Value::Object(mut value)) = serde_json::to_value(value) else {
        unreachable!()
    };
    let Some(serde_json::Value::String(tag)) = value.remove(tag) else {
        unreachable!()
    };
    (
        tag,
        value
            .remove("payload")
            .unwrap_or(serde_json::Value::Null)
            .to_string(),
    )
}

#[inline]
fn matches_json(payload: &str, expected: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload).is_ok_and(|payload| {
        serde_json::from_str::<serde_json::Value>(expected)
            .is_ok_and(|expected| payload == expected)
    })
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
    use defuse_wallet::AccountId;
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    /// Known-answer vectors: pin the types used on-chain.
    /// Recomputing them differently is a breaking change to the wire format.
    #[rstest]
    #[case(
        Eip712RequestMessage::ENCODE_TYPE,
        hex!("547aceeba23d203514552238ee8823c6e7e6814bfbb1e6e8a27e81d6daa647ef")
    )]
    #[case(
        Eip712AuthMessage::ENCODE_TYPE,
        hex!("ac4a66d331a12e85c8b62dba6474cc9d52c755fa73733f4e240c8d55275b1ab7")
    )]
    #[case(
        Eip712WalletOp::ENCODE_TYPE,
        hex!("0f38fbf4200e398513656e5bc84307fbc293cbd4e127ac9933efc5bdfa12e90e")
    )]
    #[case(
        Eip712NearPromise::ENCODE_TYPE,
        hex!("36e572508f92fd050ca143bc789ee5ed2fae8b9a957a2dadf570fae1f4bfbd43")
    )]
    #[case(
        Eip712NearAction::ENCODE_TYPE,
        hex!("ab11cc9fe3361446286c15450dcef46668a54853d2430d3b16f3bd7711e34ada")
    )]
    fn type_hash(#[case] encode_type: &str, #[case] expected: Hash) {
        assert_eq!(Eip712::type_hash(encode_type), expected);
    }

    /// Leaf `payload`s are compared semantically: whitespace and key order
    /// are free, but any extra field the contract would ignore invalidates
    /// the proof.
    #[rstest]
    #[case::canonical(r#"{"account_id":"extension.near"}"#, true)]
    #[case::pretty("{\n  \"account_id\": \"extension.near\"\n}", true)]
    #[case::extra_field(r#"{"account_id":"extension.near","note":"looks safe"}"#, false)]
    #[case::other_value(r#"{"account_id":"eve.near"}"#, false)]
    #[case::not_json("add extension.near", false)]
    fn payload_semantic_comparison(#[case] payload: &str, #[case] matches: bool) {
        let op = WalletOp::add_extension("extension.near".parse::<AccountId>().unwrap());

        assert_eq!(
            Eip712WalletOp {
                op: "add_extension".to_string(),
                payload: payload.to_string(),
            }
            .matches(&op),
            matches,
        );
    }
}
