use std::collections::{BTreeMap, BTreeSet};

use near_sdk::{
    AccountId, AccountIdRef,
    borsh::{self, BorshSerialize},
    near,
};

pub const STATE_KEY: &[u8] = b"";

/// State of the wallet-contract.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State<PubKey, Nonces> {
    /// Whether authentication by signature is allowed.
    pub signature_enabled: bool,

    /// Subwallet id: enables a single public key to have multiple different
    /// wallet-contracts.
    pub wallet_id: u32,

    /// Public key of the signer (depends on the signature scheme being
    /// being used by the implementation)
    pub public_key: PubKey,

    #[serde(flatten)]
    pub nonces: Nonces,

    /// A set of enabled extensions.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub extensions: BTreeSet<AccountId>,
}

impl<PubKey, Nonces> State<PubKey, Nonces> {
    pub const DEFAULT_WALLET_ID: u32 = 0;

    /// Create a default state with given public key.
    #[inline]
    pub fn new(public_key: PubKey) -> Self
    where
        Nonces: Default,
    {
        Self {
            signature_enabled: true,
            wallet_id: Self::DEFAULT_WALLET_ID,
            public_key,
            nonces: Default::default(),
            extensions: BTreeSet::new(),
        }
    }

    /// Set `subwallet_id` instead of the [default](`State::DEFAULT_WALLET_ID`) one
    #[must_use]
    #[inline]
    pub const fn wallet_id(mut self, wallet_id: u32) -> Self {
        self.wallet_id = wallet_id;
        self
    }

    /// Enable extensions with given `account_ids`.
    #[must_use]
    #[inline]
    pub fn extensions(
        mut self,
        account_ids: impl IntoIterator<Item = impl Into<AccountId>>,
    ) -> Self {
        self.extensions
            .extend(account_ids.into_iter().map(Into::into));
        self
    }

    /// Returns whether authentication by signature is allowed
    #[inline]
    pub fn is_signature_allowed(&self) -> bool {
        // allow contract to work if it was mistakenly deployed with
        // `!signature_enabled` and empty extensions.
        self.signature_enabled || self.extensions.is_empty()
    }

    /// Returns whether the extension with given `account_id` is enabled.
    #[inline]
    pub fn has_extension(&self, account_id: impl AsRef<AccountIdRef>) -> bool {
        self.extensions.contains(account_id.as_ref())
    }

    /// Returns initialization state for Deterministic `AccountId` derivation
    /// as per NEP-616.
    #[inline]
    pub fn as_storage(&self) -> BTreeMap<Vec<u8>, Vec<u8>>
    where
        PubKey: BorshSerialize,
        Nonces: BorshSerialize,
    {
        [(
            STATE_KEY.to_vec(),
            borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
