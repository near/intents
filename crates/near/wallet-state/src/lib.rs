#[cfg(feature = "borsh")]
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Duration;

pub use near_account_id::{AccountId, AccountIdRef};

mod nonces;
#[cfg(feature = "concurrent")]
pub use nonces::ConcurrentNonces;
pub use nonces::Nonces;

pub const STATE_KEY: &[u8] = b"";
pub const DEFAULT_WALLET_ID: u32 = 0;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60 * 60); // 1h

/// Error type for nonce operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NoncesError {
    #[error("already executed")]
    AlreadyExecuted,
    #[error("expired or from the future")]
    ExpiredOrFuture,
}

/// State of the wallet-contract.
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State<PubKey> {
    pub signature_enabled: bool,
    pub wallet_id: u32,
    pub public_key: PubKey,
    pub nonces: Nonces,
    pub extensions: BTreeSet<AccountId>,
}

impl<PubKey> State<PubKey> {
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

    #[must_use]
    #[inline]
    pub const fn wallet_id(mut self, wallet_id: u32) -> Self {
        self.wallet_id = wallet_id;
        self
    }

    #[must_use]
    #[inline]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.nonces = Nonces::new(timeout);
        self
    }

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

    #[inline]
    pub fn is_signature_allowed(&self) -> bool {
        self.signature_enabled || self.extensions.is_empty()
    }

    #[inline]
    pub fn has_extension(&self, account_id: impl AsRef<AccountIdRef>) -> bool {
        self.extensions.contains(account_id.as_ref())
    }

    #[cfg(feature = "borsh")]
    #[inline]
    pub fn as_storage(&self) -> BTreeMap<Vec<u8>, Vec<u8>>
    where
        PubKey: ::borsh::BorshSerialize,
    {
        [(
            STATE_KEY.to_vec(),
            ::borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
