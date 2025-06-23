use defuse_bitmap::{U248, U256};
use defuse_core::{Nonces, crypto::PublicKey};
use impl_tools::autoimpl;
use near_sdk::{
    near,
    store::{IterableSet, LookupMap},
};

use crate::contract::accounts::{Account, AccountState};

/// Legacy version of [`Account`]
#[derive(Debug)]
#[near(serializers = [borsh])]
#[autoimpl(Deref using self.state)]
#[autoimpl(DerefMut using self.state)]
pub struct AccountV0 {
    pub(super) nonces: Nonces<LookupMap<U248, U256>>,

    pub(super) implicit_public_key_removed: bool,
    pub(super) public_keys: IterableSet<PublicKey>,

    pub state: AccountState,

    pub(super) prefix: Vec<u8>,
}

impl From<AccountV0> for Account {
    fn from(
        AccountV0 {
            nonces,
            implicit_public_key_removed,
            public_keys,
            state,
            prefix,
        }: AccountV0,
    ) -> Self {
        Self {
            nonces,
            implicit_public_key_removed,
            public_keys,
            state,
            prefix,
        }
    }
}
