use std::collections::{BTreeMap, BTreeSet};

use near_sdk::{
    AccountId, AccountIdRef,
    borsh::{self, BorshSerialize},
    near,
};

/// State of the wallet-contract.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State<P> {
    /// Whether authentication by signature is allowed.
    pub signature_enabled: bool,

    /// Next valid seqno (i.e. nonce).
    pub seqno: u32,

    /// Subwallet id: enables a single public key to have multiple different
    /// wallet-contracts.
    pub wallet_id: u32,

    /// Public key of the signer (depends on the signature scheme being
    /// being used by the implementation)
    pub public_key: P,

    /// A set of enabled extensions.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub extensions: BTreeSet<AccountId>,
}

impl<P> State<P> {
    pub const DEFAULT_WALLET_ID: u32 = 0;

    pub(crate) const STATE_KEY: &[u8] = b"";

    /// Create a default state with given public key.
    #[inline]
    pub const fn new(public_key: P) -> Self {
        Self {
            signature_enabled: true,
            seqno: 0,
            wallet_id: Self::DEFAULT_WALLET_ID,
            public_key,
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

    /// Allow contract to work if it was mistakenly deployed with
    /// `!signature_enabled` and empty extensions.
    #[inline]
    pub fn is_signature_allowed(&self) -> bool {
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
    pub fn init_state(&self) -> BTreeMap<Vec<u8>, Vec<u8>>
    where
        P: BorshSerialize,
    {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
