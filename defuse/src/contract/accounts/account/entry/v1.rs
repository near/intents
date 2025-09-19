use defuse_bitmap::{U248, U256};
use defuse_core::{Nonces, crypto::PublicKey};
use defuse_near_utils::NestPrefix;
use impl_tools::autoimpl;
use near_sdk::{
    near,
    store::{IterableSet, LookupMap},
};

use crate::contract::accounts::{
    Account, AccountState, MaybeOptimizedNonces,
    account::{AccountFlags, AccountPrefix},
};

/// Legacy: V1 of [`Account`]
#[derive(Debug)]
#[near(serializers = [borsh])]
#[autoimpl(Deref using self.state)]
#[autoimpl(DerefMut using self.state)]
pub struct AccountV1 {
    pub(super) nonces: Nonces<LookupMap<U248, U256>>,

    pub(super) flags: AccountFlags,
    pub(super) public_keys: IterableSet<PublicKey>,

    pub state: AccountState,

    pub(super) prefix: Vec<u8>,
}

impl From<AccountV1> for Account {
    fn from(
        AccountV1 {
            nonces,
            flags,
            public_keys,
            state,
            prefix,
        }: AccountV1,
    ) -> Self {
        Self {
            nonces: MaybeOptimizedNonces::new_with_legacy(
                prefix.as_slice().nest(AccountPrefix::OptimizedNonces),
                nonces,
            ),
            flags,
            public_keys,
            state,
            prefix,
        }
    }
}
