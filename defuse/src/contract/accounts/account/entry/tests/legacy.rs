use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use arbitrary_with::{Arbitrary, As, arbitrary};
use defuse_bitmap::U256;
use defuse_borsh_utils::adapters::to_vec_as;
use defuse_core::{Result, crypto::PublicKey, token_id::TokenId};
use defuse_near_utils::{Lock, PanicOnClone, arbitrary::ArbitraryAccountId};
use defuse_test_utils::random::make_arbitrary;
use near_sdk::{
    AccountId,
    borsh::{self, BorshDeserialize, BorshSerialize},
};
use rstest::rstest;

use crate::contract::accounts::{
    Account,
    account::{
        AccountEntry,
        entry::{AccountV0, MaybeVersionedAccountEntry, VersionedAccountEntry, v1::AccountV1},
        nonces::tests::random_nonces,
    },
};

fn deserialize_and_check_legacy_account(
    serialized_legacy: &[u8],
    data: &AccountData,
    random_nonces: &[U256],
) {
    let mut versioned: AccountEntry = borsh::from_slice(serialized_legacy).unwrap();

    let account = versioned
        .lock()
        .expect("legacy accounts must be unlocked by default");
    data.assert_contained_in(account);

    // commit new nonces
    for nonce in random_nonces {
        assert!(account.commit_nonce(*nonce).is_ok());
    }

    let serialized_versioned = borsh::to_vec(&versioned).unwrap();
    drop(versioned);

    let versioned: AccountEntry = borsh::from_slice(&serialized_versioned).unwrap();
    let account = versioned
        .as_locked()
        .expect("legacy accounts must be unlocked by default");
    data.assert_contained_in(account);

    // check new nonces existence
    for &n in random_nonces {
        assert!(account.is_nonce_used(n));
    }
}

#[rstest]
fn legacy_upgrade(#[from(make_arbitrary)] data: AccountData, random_nonces: Vec<U256>) {
    // legacy accounts have no wrappers around them
    let legacy_acc = data.make_legacy_account::<AccountV0>();
    let serialized_legacy = borsh::to_vec(&legacy_acc).expect("unable to serialize legacy Account");

    // we need to drop it, so all collections from near-sdk flush to storage
    drop(legacy_acc);

    deserialize_and_check_legacy_account(&serialized_legacy, &data, &random_nonces);
}

#[rstest]
#[case::v0(PhantomData::<Lock<AccountV1>>)]
#[allow(clippy::used_underscore_binding)]
fn versioned_upgrade<T>(
    #[from(make_arbitrary)] data: AccountData,
    random_nonces: Vec<U256>,
    #[case] _marker: PhantomData<T>,
) where
    T: LegacyAccountBuilder + BorshSerialize + BorshDeserialize,
    for<'a> VersionedAccountEntry<'a>: From<&'a T>,
{
    // versioned accounts always have wrappers around them and should be serialized with prefix

    let legacy_entry = data.make_legacy_account::<T>();
    let serialized_legacy = to_vec_as::<_, MaybeVersionedAccountEntry>(&legacy_entry)
        .expect("unable to serialize legacy Account");

    // we need to drop it, so all collections from near-sdk flush to storage
    drop(legacy_entry);

    deserialize_and_check_legacy_account(&serialized_legacy, &data, &random_nonces);
}

/// Data for legacy account creating
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
    fn make_legacy_account<B: LegacyAccountBuilder>(&self) -> B {
        let mut legacy = B::new(self.prefix.as_slice(), &self.account_id);

        for pubkey in &self.public_keys {
            assert!(legacy.add_public_key(&self.account_id, *pubkey));
        }

        if let Some(pk) = PublicKey::from_implicit_account_id(&self.account_id)
            .filter(|_| self.try_remove_implicit_public_key)
        {
            assert!(legacy.remove_public_key(&self.account_id, &pk));
        }

        for nonce in &self.nonces {
            assert!(legacy.commit_nonce(*nonce).is_ok());
        }

        for (token_id, &amount) in &self.token_balances {
            assert!(legacy.add_balance(token_id.clone(), amount));
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

trait LegacyAccountBuilder {
    fn new(prefix: &[u8], account_id: &AccountId) -> Self;
    fn add_public_key(&mut self, account_id: &AccountId, pk: PublicKey) -> bool;
    fn remove_public_key(&mut self, account_id: &AccountId, pk: &PublicKey) -> bool;
    fn commit_nonce(&mut self, nonce: U256) -> Result<()>;
    fn add_balance(&mut self, token_id: TokenId, amount: u128) -> bool;
}

// Added macro for builder implementation to reduce boilerplate
macro_rules! impl_legacy_account_builder {
    ($account_type:ty) => {
        impl LegacyAccountBuilder for $account_type {
            fn new(prefix: &[u8], account_id: &AccountId) -> Self {
                <$account_type>::new(prefix, account_id)
            }

            fn add_public_key(&mut self, account_id: &AccountId, pk: PublicKey) -> bool {
                self.add_public_key(account_id, pk)
            }

            fn remove_public_key(&mut self, account_id: &AccountId, pk: &PublicKey) -> bool {
                self.remove_public_key(account_id, pk)
            }

            fn commit_nonce(&mut self, nonce: U256) -> Result<()> {
                self.commit_nonce(nonce)
            }

            fn add_balance(&mut self, token_id: TokenId, amount: u128) -> bool {
                self.token_balances.add(token_id, amount).is_some()
            }
        }
    };
}

macro_rules! impl_lock_account_builder {
    ($account_type:ty) => {
        impl LegacyAccountBuilder for Lock<$account_type> {
            fn new(prefix: &[u8], account_id: &AccountId) -> Self {
                Lock::unlocked(<$account_type>::new(prefix, account_id))
            }

            fn add_public_key(&mut self, account_id: &AccountId, pk: PublicKey) -> bool {
                self.get_mut().unwrap().add_public_key(account_id, pk)
            }

            fn remove_public_key(&mut self, account_id: &AccountId, pk: &PublicKey) -> bool {
                self.get_mut().unwrap().remove_public_key(account_id, pk)
            }

            fn commit_nonce(&mut self, nonce: U256) -> Result<()> {
                self.get_mut().unwrap().commit_nonce(nonce)
            }

            fn add_balance(&mut self, token_id: TokenId, amount: u128) -> bool {
                self.get_mut()
                    .unwrap()
                    .token_balances
                    .add(token_id, amount)
                    .is_some()
            }
        }
    };
}

impl_legacy_account_builder!(AccountV0);
impl_lock_account_builder!(AccountV1);

impl<'a> From<&'a Lock<AccountV1>> for VersionedAccountEntry<'a> {
    fn from(value: &'a Lock<AccountV1>) -> Self {
        Self::V1(Cow::Borrowed(PanicOnClone::from_ref(value)))
    }
}
