use defuse_nep641::OffchainMessage;
use near_account_id::AccountId;

use crate::RequestMessage;

/// Signature schema used by [`Wallet`](crate::contract::Wallet) contract
/// variant.
///
/// By design, each wallet contract variant implements its own schema and
/// gets deployed separately.
pub trait SignatureSchema {
    /// Public key used by this schema and [stored](field@crate::State::public_key)
    /// in the contract's state.
    ///
    ///
    /// Its [`Display`](core::fmt::Display) implementation is returned from
    /// [`w_public_key()`](crate::contract::Wallet::w_public_key) contract
    /// method.
    type PublicKey;

    /// Verify given proof over the request message in respect to the
    /// public key and return whether verification passed.
    ///
    /// Used by the `w_execute_signed(msg, proof)` contract method.
    #[must_use = "check if verification passed"]
    fn verify_request_msg(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool;

    // TODO: docs, naming
    #[must_use = "check if verification passed"]
    fn verify_offchain_msg(
        public_key: &Self::PublicKey,
        msg: &OffchainMessage,
        proof: &str,
    ) -> bool;
}

/// NEP-641 authorization for [`Wallet`](crate::contract::Wallet) contract.
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WalletAuthorization {
    Signature {
        msg: OffchainMessage,
        proof: String,
    },
    Extension {
        account_id: AccountId,
        authorization: String,
        payload: String,
    },
}

impl WalletAuthorization {
    /// Get the authorized payload
    #[inline]
    pub const fn payload(&self) -> &str {
        match self {
            Self::Signature {
                msg: OffchainMessage { payload, .. },
                ..
            } => payload.as_str(),
            Self::Extension { payload, .. } => payload.as_str(),
        }
    }

    /// Convert into the authorized payload
    #[inline]
    pub fn into_payload(self) -> String {
        match self {
            Self::Signature {
                msg: OffchainMessage { payload, .. },
                ..
            } => payload,
            Self::Extension { payload, .. } => payload,
        }
    }

    /// Convert to the authorization blob
    #[cfg(feature = "json")]
    #[inline]
    pub fn to_authorization(&self) -> String {
        serde_json::to_string(self).expect("JSON: failed to serialize")
    }

    /// Wrap as extension with given ID
    #[cfg(feature = "json")]
    #[inline]
    pub fn as_extension_of(self, account_id: impl Into<AccountId>) -> Self {
        Self::Extension {
            account_id: account_id.into(),
            authorization: self.to_authorization(),
            payload: self.into_payload(),
        }
    }
}
