use std::collections::{HashMap, HashSet};

use arbitrary_with::{Arbitrary, As, arbitrary};
use defuse_bitmap::U256;
use defuse_core::{
    Nonces, Result,
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
    token_id::TokenId,
};
use defuse_near_utils::{NestPrefix, arbitrary::ArbitraryAccountId};
use defuse_test_utils::random::make_arbitrary;
use near_sdk::{
    AccountId,
    borsh::{self},
};
use rstest::rstest;

use crate::contract::accounts::{
    Account,
    account::{AccountEntry, entry::AccountV0},
};

#[rstest]
fn legacy_upgrade(#[from(make_arbitrary)] data: AccountData) {
    let legacy = data.make_legacy_account();
    let serialized_legacy = borsh::to_vec(&legacy).expect("unable to serialize legacy Account");
    // we need to drop it, so all collections from near-sdk flush to storage
    drop(legacy);

    let mut versioned: AccountEntry = borsh::from_slice(&serialized_legacy).unwrap();
    data.assert_contained_in(
        versioned
            .lock()
            .expect("legacy accounts must be unlocked by default"),
    );
    let serialized_versioned = borsh::to_vec(&versioned).unwrap();
    drop(versioned);

    let versioned: AccountEntry = borsh::from_slice(&serialized_versioned).unwrap();
    data.assert_contained_in(versioned.as_locked().expect("should be locked by now"));
}

/// Data for creating [`AccountV0`]
#[derive(Arbitrary)]
struct AccountData {
    prefix: Vec<u8>,
    #[arbitrary(with = As::<ArbitraryAccountId>::arbitrary)]
    account_id: AccountId,

    public_keys: HashSet<PublicKey>,
    try_remove_implicit_public_key: bool,
    nonces: HashSet<U256>,
    token_balances: HashMap<TokenId, u128>,
}

impl AccountData {
    fn make_legacy_account(&self) -> AccountV0 {
        let mut legacy = AccountV0::new(self.prefix.as_slice(), &self.account_id);

        for &pk in &self.public_keys {
            assert!(legacy.add_public_key(&self.account_id, pk));
        }
        if let Some(implicit_pk) = PublicKey::from_implicit_account_id(&self.account_id)
            .filter(|_| self.try_remove_implicit_public_key)
        {
            assert!(legacy.remove_public_key(&self.account_id, &implicit_pk));
        }

        for &n in &self.nonces {
            assert!(legacy.commit_nonce(n).is_ok());
        }

        for (token_id, &amount) in &self.token_balances {
            assert!(
                legacy
                    .token_balances
                    .add(token_id.clone(), amount)
                    .is_some()
            );
        }

        legacy
    }

    fn assert_contained_in(&self, a: &Account) {
        for pk in &self.public_keys {
            assert!(a.has_public_key(&self.account_id, pk));
        }

        for &n in &self.nonces {
            assert!(a.is_nonce_used(n));
        }

        for (token_id, &amount) in &self.token_balances {
            assert_eq!(a.token_balances.amount_for(token_id), amount);
        }
    }
}
