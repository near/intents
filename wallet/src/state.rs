use std::collections::{BTreeMap, BTreeSet};

use near_sdk::{
    AccountId, AccountIdRef,
    borsh::{self, BorshSerialize},
    near,
};

use crate::SigningStandard;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State<S: SigningStandard> {
    pub signature_enabled: bool,
    pub seqno: u32,
    pub wallet_id: u32,
    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(with = "String"),
        borsh(schema(params = "S => <S as SigningStandard>::PublicKey"))
    )]
    // TODO: serde_as?
    pub public_key: S::PublicKey,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub extensions: BTreeSet<AccountId>,
}

impl<S: SigningStandard> State<S> {
    pub const DEFAULT_WALLET_ID: u32 = 0;

    pub(crate) const STATE_KEY: &[u8] = b"";

    #[inline]
    pub const fn new(public_key: S::PublicKey) -> Self {
        Self {
            signature_enabled: true,
            seqno: 0,
            wallet_id: Self::DEFAULT_WALLET_ID,
            public_key,
            extensions: BTreeSet::new(),
        }
    }

    #[inline]
    pub fn wallet_id(mut self, wallet_id: u32) -> Self {
        self.wallet_id = wallet_id;
        self
    }

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
    /// auth_by_signature_disabled and empty extensions.
    #[inline]
    pub fn is_signature_allowed(&self) -> bool {
        self.signature_enabled || self.extensions.is_empty()
    }

    #[inline]
    pub fn has_extension(&self, account_id: impl AsRef<AccountIdRef>) -> bool {
        self.extensions.contains(account_id.as_ref())
    }

    #[inline]
    pub fn init_state(&self) -> BTreeMap<Vec<u8>, Vec<u8>>
    where
        S::PublicKey: BorshSerialize,
    {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}
