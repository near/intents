use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use near_sdk::{
    AccountId, AccountIdRef,
    borsh::{self, BorshSerialize},
    near,
};

use crate::Nonces;

pub const STATE_KEY: &[u8] = b"";

pub const DEFAULT_WALLET_ID: u32 = 0;

/// Recommended timeout for production use: `1 hour`.
///
/// NOTE: The longer timeout, the more storage usage in highload environments.
/// **Theoretically**, in order to fit into ZBA limits while sending 1 tx/sec
/// over the timespan of `2 * timeout`, timeout should be at most `15m`.
///
/// However, we should take into account that 30% of used gas is funnelled back
/// to the contract's NEAR balance. Assuming that `w_execute_signed()` method
/// uses at least 2TGas, at the time of writing this converts back to ~0.03
/// milliNear, which is enough to cover storage staking fees for 3 bytes.
///
/// If nonces are generated optimally (i.e. at most
/// [semi-sequentially](crate::ConcurrentNonces)), then each
/// nonce consumes ~2 bits on average. So, each nonce committed in
/// `w_execute_signed()` brings us more NEAR than we would have to reserve for
/// storage staking if we ever exceed ZBA limits.
///
/// As a result, `1h` is an acceptable timeout allowing messages to survive
/// relayer/chain congestion.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60 * 60); // 1h

/// State of the wallet-contract.
#[near(serializers = [borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State<PubKey> {
    /// Whether authentication by signature is allowed.
    pub signature_enabled: bool,

    /// Subwallet id: enables a single public key to have multiple different
    /// wallet-contracts.
    pub wallet_id: u32,

    /// Public key of the signer (depends on the signature scheme being
    /// being used by the implementation)
    pub public_key: PubKey,

    /// Set of used timeout-based nonces.
    pub nonces: Nonces,

    /// A set of enabled extensions.
    pub extensions: BTreeSet<AccountId>,
}

impl<PubKey> State<PubKey> {
    /// Create a default state with given public key.
    #[inline]
    pub const fn new(public_key: PubKey) -> Self {
        Self {
            signature_enabled: true,
            wallet_id: DEFAULT_WALLET_ID,
            public_key,
            nonces: Nonces::new(DEFAULT_TIMEOUT),
            extensions: BTreeSet::new(),
        }
    }

    /// Set given `subwallet_id` instead of the
    /// [default](`State::DEFAULT_WALLET_ID`) one.
    /// This can be used to derive multiple wallet-contract from a single
    /// public key.
    #[must_use]
    #[inline]
    pub const fn wallet_id(mut self, wallet_id: u32) -> Self {
        self.wallet_id = wallet_id;
        self
    }

    /// Set given `timeout` instead of default the
    /// [default](`State::DEFAULT_TIMEOUT`) one.
    #[must_use]
    #[inline]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.nonces = Nonces::new(timeout);
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
