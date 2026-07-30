//! [NEP-641](https://github.com/near/NEPs/blob/master/neps/nep-0641.md)
//! off-chain authorization resolution.
//!
//! See [`w_resolve_auth()`](crate::contract::Wallet::w_resolve_auth).

use core::time::Duration;
use std::collections::BTreeSet;

use defuse_time::Timestamp;
use near_account_id::AccountId;

#[cfg(feature = "borsh")]
use ::{
    defuse_borsh_utils::{As, DurationSeconds as BorshDurationSeconds},
    defuse_time::borsh::TimestampNanoSeconds,
};
#[cfg(feature = "arbitrary")]
use defuse_time::arbitrary::RangeNanos;
#[cfg(feature = "serde")]
use serde_with::DurationSeconds;

/// Domain prefix for signing [`AuthMessage`].
///
/// This prefix doesn't break NEP-461 assumptions, since first four bytes
/// borsh-deserialize to `1380009294u32`, which is in `[1 << 30, 1 << 31)`
/// range for on-chain messages and is not an allocated NEP discriminant.
///
/// Although it shares the `NEAR_WALLET_CONTRACT` prefix with
/// [`WALLET_DOMAIN`](crate::WALLET_DOMAIN), the domains are unambiguous:
/// the byte following the shared prefix differs (`/` vs `_`), and a borsh
/// payload following either domain cannot legally re-encode the other's
/// suffix (the leading `chain_id` length prefix would have to be over 1GB).
pub const WALLET_AUTH_DOMAIN: &[u8] = b"NEAR_WALLET_CONTRACT_AUTH/V1";

/// Signable authorization message resolved via
/// [`w_resolve_auth()`](crate::contract::Wallet::w_resolve_auth) contract
/// method.
///
/// # Replay protection
///
/// Unlike [`RequestMessage`](crate::RequestMessage), this message doesn't
/// contain a nonce, since `w_resolve_auth()` is a stateless view method and
/// cannot commit nonces. Replay protection is layered instead:
/// * within a dApp: the dApp MUST issue a fresh, unique [`payload`](Self::payload)
///   per authorization and only accept payloads it has issued (see NEP-641),
/// * across dApps: [`recipient`](Self::recipient) binding,
/// * across purposes: [`purpose`](Self::purpose) binding,
/// * across networks: [`chain_id`](Self::chain_id) binding,
/// * across accounts: [`signer`](Self::signer) binding,
/// * over time: [`created_at`](Self::created_at)/[`timeout`](Self::timeout)
///   validity window.
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuthMessage {
    /// Chain id (e.g. `mainnet`).
    /// MUST be equal to `chain_id` of the network.
    pub chain_id: String,

    /// Binding to the wallet-contract instance this authorization is for.
    pub signer: AuthSignerBinding,

    /// MUST be equal to the `purpose` argument of `w_resolve_auth()`.
    pub purpose: String,

    /// MUST be equal to the `recipient` argument of `w_resolve_auth()`.
    pub recipient: String,

    /// dApp-issued payload, returned verbatim on successful resolution.
    pub payload: String,

    #[cfg_attr(
        feature = "arbitrary",
        arbitrary(with = ::arbitrary_with::As::<RangeNanos::<0>>::arbitrary),
    )]
    #[cfg_attr(
        feature = "borsh-schema",
        borsh(
            serialize_with = "As::<TimestampNanoSeconds<u64>>::serialize",
            deserialize_with = "As::<TimestampNanoSeconds<u64>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampNanoSeconds<u64>>::add_definitions_recursively",
                declaration = "As::<TimestampNanoSeconds<u64>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        all(feature = "borsh", not(feature = "borsh-schema")),
        borsh(
            serialize_with = "As::<TimestampNanoSeconds<u64>>::serialize",
            deserialize_with = "As::<TimestampNanoSeconds<u64>>::deserialize",
        )
    )]
    /// Timestamp when this authorization was created (in RFC-3339 format).
    ///
    /// Clients are recommended to set `created_at` slightly (e.g. 60 seconds)
    /// before the actual time of signing, so that verification doesn't fail
    /// on-chain due to lagging block timestamps.
    pub created_at: Timestamp,

    #[cfg_attr(
        feature = "borsh-schema",
        borsh(
            serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
            deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<BorshDurationSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<BorshDurationSeconds<u32>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        all(feature = "borsh", not(feature = "borsh-schema")),
        borsh(
            serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
            deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
        )
    )]
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "DurationSeconds"),
        serde(rename = "timeout_secs")
    )]
    /// Maximum timeout for validity of this authorization after `created_at`.
    /// The actual timeout is `min(msg.timeout, contract.timeout)`.
    pub timeout: Duration,
}

impl AuthMessage {
    /// Returns canonical hash of the authorization message:
    ///
    /// ```text
    /// SHA3-256(b"NEAR_WALLET_CONTRACT_AUTH/V1" || borsh(msg))
    /// ```
    ///
    /// This hash is what gets signed by the wallet's key. For `WebAuthn`
    /// schemas, it is used as the challenge of the assertion.
    #[cfg(all(feature = "digest", feature = "borsh"))]
    pub fn hash(&self) -> [u8; 32] {
        use defuse_digest::{Digest, sha3::Sha3_256};
        use digest_io::IoWrapper;

        let mut hasher = IoWrapper(Sha3_256::new_with_prefix(WALLET_AUTH_DOMAIN));
        // serialize directly to hasher
        ::borsh::to_writer(&mut hasher, self).expect("borsh: failed to serialize");

        hasher.0.finalize().into()
    }
}

/// Binding of an [`AuthMessage`] to the wallet-contract instance it
/// authorizes for.
///
/// Prevents an authorization signed for one wallet-contract account from
/// being replayed against another account controlled by the same key.
///
/// # `deny_unknown_fields`
///
/// Deserialization is strict: a binding carrying any field this schema does
/// not know is rejected. This lets a FUTURE wallet-contract variant of the
/// **same signature curve** be added to a [`Code`](Self::Code) binding's
/// [`allowed_factory_ids`](Self::Code::allowed_factory_ids) without
/// re-introducing cross-account replay — provided that variant's `Code`
/// binding carries a DIFFERENT (e.g. added, required) field. A message shaped
/// for the new variant then fails to parse on the old contract (unknown
/// field), and a message shaped for the old variant fails to parse on the new
/// one (missing required field), so no single signed message resolves under
/// both. Without this, serde would silently ignore the extra field and the
/// old contract would accept the new variant's message — the very replay the
/// per-curve invariant exists to prevent.
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(tag = "type", rename_all = "snake_case", deny_unknown_fields),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    borsh(use_discriminant = true),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AuthSignerBinding {
    /// Conventional NEP-641 binding to an exact account id.
    ///
    /// MUST be equal to `env::current_account_id()`.
    ///
    /// Clients SHOULD prefer this binding whenever the account id is
    /// already known.
    SignerId {
        /// `AccountId` of the wallet-contract instance.
        signer_id: AccountId,
    } = 0,

    /// Binding to the deterministic account id (NEP-616) via its
    /// `StateInit`: the *config* part of the **initial**
    /// [`State`](crate::State).
    ///
    /// The contract reconstructs
    /// `StateInit { code: env::current_global_contract_id(), data:
    /// State { ..config, public_key } }` — taking the code identity from
    /// the code it is currently running under and `public_key` from its
    /// own storage — derives the deterministic account id from it and
    /// verifies it equals `env::current_account_id()`. A match proves the
    /// envelope was intended for this exact account: the account id
    /// commits to the code, the full initial config and the public key
    /// (which the signature additionally binds).
    ///
    /// The binding stays constructible client-side *before* the signing
    /// ceremony reveals which key answers (e.g. `WebAuthn` passkey
    /// discovery) — it doesn't even depend on which wallet-contract
    /// variant the credential maps to.
    ///
    /// Because the binding commits to the account's *initial* state (which
    /// determines its account id forever), post-creation config mutations
    /// (added extensions, signature-mode changes) do NOT invalidate it.
    ///
    /// The envelope pins the set of *canonical factory code identities* it may
    /// resolve under ([`allowed_factory_ids`](Self::Code::allowed_factory_ids)),
    /// which — together with the derived-account-id check — collapses the set
    /// of accounts a single signed message can authorize down to exactly one
    /// per curve. See that field's docs for the invariant that MUST hold.
    Code {
        /// The canonical wallet-contract factory account ids (NEP-591 global
        /// contracts deployed by account id) this authorization may resolve
        /// under. Resolution succeeds only if the code this contract is
        /// currently running under is one of these — a signed, explicit
        /// allow-list of canonical factories that rejects any other code
        /// identity (a rogue or not-yet-declared factory) as
        /// [`SignerBindingMismatch`](crate::AuthError::SignerBindingMismatch).
        ///
        /// The client can populate this *before* the signing ceremony reveals
        /// which curve (and thus which factory) answers, because all canonical
        /// factory ids are fixed constants independent of the curve — each
        /// per-curve wallet-contract instance enforces only its own membership.
        ///
        /// # Invariant (MUST hold)
        ///
        /// `allowed_factory_ids` MUST NOT contain two factories of the **same
        /// signature curve**. The signed message commits to the *set*, not to
        /// which member answered; a signature verifies under every factory of
        /// its own curve that appears here. So if two same-curve factories were
        /// listed and the key holder had an account under each, the **same
        /// signed message would resolve `RESOLVED` against both accounts** — a
        /// cross-account replay. With at most one factory per curve, the
        /// signature's own curve selects a unique factory, and the set is a
        /// singleton per curve.
        allowed_factory_ids: BTreeSet<AccountId>,

        /// [`State::signature_enabled`](field@crate::State::signature_enabled)
        /// the account was initialized with.
        signature_enabled: bool,

        /// [`State::subwallet_id`](field@crate::State::subwallet_id)
        /// the account was initialized with.
        subwallet_id: u32,

        #[cfg_attr(
            feature = "borsh-schema",
            borsh(
                serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
                deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
                schema(with_funcs(
                    definitions = "As::<BorshDurationSeconds<u32>>::add_definitions_recursively",
                    declaration = "As::<BorshDurationSeconds<u32>>::declaration",
                ))
            )
        )]
        #[cfg_attr(
            all(feature = "borsh", not(feature = "borsh-schema")),
            borsh(
                serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
                deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
            )
        )]
        #[cfg_attr(
            feature = "serde",
            serde_as(as = "DurationSeconds"),
            serde(rename = "timeout_secs")
        )]
        /// [`Nonces::timeout()`](crate::Nonces::timeout) the account was
        /// initialized with (initial nonce bitmaps are always empty, so
        /// the timeout fully determines the initial [`Nonces`](crate::Nonces)).
        timeout: Duration,

        /// [`State::extensions`](field@crate::State::extensions) the
        /// account was initialized with.
        extensions: BTreeSet<AccountId>,
    } = 1,
}

/// Result of [`w_resolve_auth()`](crate::contract::Wallet::w_resolve_auth),
/// serialized exactly as specified by NEP-641.
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE"),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationResolution {
    /// The authorization is valid for this account.
    Resolved {
        /// The unwrapped, original payload issued by the dApp.
        payload: String,
    },

    /// The authorization requires further resolution on other accounts.
    ///
    /// Single-signer wallet contracts NEVER return this variant; it is part
    /// of this type only to represent the full NEP-641 wire format
    /// (e.g. when deserializing responses from multisig contracts).
    Pending {
        /// The unwrapped, original payload issued by the dApp.
        payload: String,
        /// Sub-authorizations the caller must recursively resolve.
        pending_authorizations: Vec<PendingAuthorization>,
    },

    /// The authorization is invalid.
    Invalid {
        /// Machine-readable error kind.
        error_kind: AuthErrorKind,
        /// Human-readable error message.
        error_message: String,
    },
}

/// Sub-authorization to be recursively resolved by the caller (NEP-641).
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingAuthorization {
    /// Account to resolve the sub-authorization on.
    pub account_id: AccountId,
    /// Purpose to resolve the sub-authorization with.
    pub purpose: String,
    /// Extracted sub-authorization blob.
    pub authorization: String,
}

/// Machine-readable error kind of
/// [`AuthorizationResolution::Invalid`] (NEP-641).
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(rename_all = "SCREAMING_SNAKE_CASE"),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthErrorKind {
    /// The authorization blob is malformed or doesn't commit to the
    /// supplied arguments / this account.
    InvalidInput,
    /// Cryptographic signature verification failed.
    InvalidSignature,
}

/// An error that can occur when resolving an authorization.
///
/// Unlike [`ContractError`](crate::ContractError), these errors are
/// returned as [`AuthorizationResolution::Invalid`] from the view method,
/// never panicked.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("authorization: {0}")]
    MalformedAuthorization(String),

    #[error("purpose mismatch")]
    PurposeMismatch,

    #[error("recipient mismatch")]
    RecipientMismatch,

    #[error("invalid chain_id")]
    InvalidChainId,

    #[error("expired or from the future")]
    ExpiredOrFuture,

    #[error("signer binding mismatch")]
    SignerBindingMismatch,

    #[error("signature is disabled")]
    SignatureDisabled,

    #[error("invalid signature")]
    InvalidSignature,
}

impl AuthError {
    /// Maps to NEP-641 [`AuthErrorKind`].
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> AuthErrorKind {
        match self {
            Self::InvalidSignature => AuthErrorKind::InvalidSignature,
            _ => AuthErrorKind::InvalidInput,
        }
    }
}

impl From<AuthError> for AuthorizationResolution {
    #[inline]
    fn from(err: AuthError) -> Self {
        Self::Invalid {
            error_kind: err.kind(),
            error_message: err.to_string(),
        }
    }
}

/// The `authorization` blob accepted by
/// [`w_resolve_auth()`](crate::contract::Wallet::w_resolve_auth),
/// passed as its JSON string representation.
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedAuthMessage {
    /// The signed authorization message.
    pub message: AuthMessage,

    /// [Schema](crate::SignatureSchema)-specific proof over
    /// [`message.hash()`](AuthMessage::hash), in the same format as the
    /// `proof` argument of
    /// [`w_execute_signed()`](crate::contract::Wallet::w_execute_signed).
    pub proof: String,
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    fn sample_msg() -> AuthMessage {
        AuthMessage {
            chain_id: "mainnet".to_string(),
            signer: AuthSignerBinding::Code {
                allowed_factory_ids: BTreeSet::from([
                    "p256-passkey-wallet-contract.trezu.near".parse().unwrap(),
                    "ed25519-passkey-wallet-contract.trezu.near"
                        .parse()
                        .unwrap(),
                ]),
                signature_enabled: true,
                subwallet_id: 0,
                timeout: Duration::from_hours(1),
                extensions: BTreeSet::new(),
            },
            purpose: "PROVE_OWNERSHIP".to_string(),
            recipient: "trezu.app".to_string(),
            payload: "Login to trezu.app at 2026-07-16T00:00:00Z".to_string(),
            created_at: Timestamp::UNIX_EPOCH,
            timeout: Duration::from_hours(1),
        }
    }

    #[rstest]
    fn code_binding_json() {
        let msg = sample_msg();
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "chain_id": "mainnet",
                "signer": {
                    "type": "code",
                    "allowed_factory_ids": [
                        "ed25519-passkey-wallet-contract.trezu.near",
                        "p256-passkey-wallet-contract.trezu.near",
                    ],
                    "signature_enabled": true,
                    "subwallet_id": 0,
                    "timeout_secs": 3600,
                    "extensions": [],
                },
                "purpose": "PROVE_OWNERSHIP",
                "recipient": "trezu.app",
                "payload": "Login to trezu.app at 2026-07-16T00:00:00Z",
                "created_at": "1970-01-01T00:00:00Z",
                "timeout_secs": 3600,
            })
        );
        let roundtrip: AuthMessage = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip, msg);
    }

    /// `deny_unknown_fields` must actually reject extra fields (serde has
    /// historically ignored it on internally-tagged enums) — this is what lets
    /// a future same-curve factory be added to `allowed_factory_ids` safely.
    #[rstest]
    fn binding_denies_unknown_field() {
        // baseline parses
        let mut json = serde_json::to_value(sample_msg()).unwrap();
        assert!(serde_json::from_value::<AuthMessage>(json.clone()).is_ok());

        // a `Code` binding carrying an unknown (future-variant) field is rejected
        json["signer"]["future_variant_field"] = serde_json::json!("x");
        assert!(
            serde_json::from_value::<AuthMessage>(json).is_err(),
            "deny_unknown_fields must reject unknown Code binding fields",
        );

        // ...and likewise for the SignerId variant
        let signer_id_json = serde_json::json!({
            "chain_id": "mainnet",
            "signer": {
                "type": "signer_id",
                "signer_id": "0s0000000000000000000000000000000000000000",
                "future_variant_field": "x",
            },
            "purpose": "PROVE_OWNERSHIP",
            "recipient": "trezu.app",
            "payload": "x",
            "created_at": "1970-01-01T00:00:00Z",
            "timeout_secs": 3600,
        });
        assert!(serde_json::from_value::<AuthMessage>(signer_id_json).is_err());
    }

    #[rstest]
    fn signer_id_binding_json() {
        let binding = AuthSignerBinding::SignerId {
            signer_id: "0s0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
        };
        let json = serde_json::to_value(&binding).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "signer_id",
                "signer_id": "0s0000000000000000000000000000000000000000",
            })
        );
    }

    /// Known-answer vector: pins the canonical borsh layout + domain prefix.
    #[rstest]
    fn hash_vector() {
        assert_eq!(
            sample_msg().hash(),
            // pinned known-answer; recomputing differently means a breaking
            // change to the wire format
            hex!("443784c84117260c1b84acc2839155df5a840ef566139251fa3f221b037cba82"),
        );
    }

    #[rstest]
    fn resolution_json() {
        assert_eq!(
            serde_json::to_string(&AuthorizationResolution::Resolved {
                payload: "hello".to_string(),
            })
            .unwrap(),
            r#"{"status":"RESOLVED","payload":"hello"}"#,
        );

        assert_eq!(
            serde_json::to_string(&AuthorizationResolution::Invalid {
                error_kind: AuthErrorKind::InvalidSignature,
                error_message: "invalid signature".to_string(),
            })
            .unwrap(),
            r#"{"status":"INVALID","error_kind":"INVALID_SIGNATURE","error_message":"invalid signature"}"#,
        );
    }
}
