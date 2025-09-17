use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::RangeBounds,
};

use arbitrary_with::{Arbitrary, As, arbitrary};
use chrono::{Days, Utc};
use defuse_bitmap::U256;
use defuse_core::{Deadline, ExpirableNonce, Result, crypto::PublicKey, token_id::TokenId};
use defuse_near_utils::arbitrary::ArbitraryAccountId;
use defuse_test_utils::random::{Rng, make_arbitrary, range_to_random_size, rng};
use near_sdk::{
    AccountId,
    borsh::{self, BorshDeserialize, BorshSerialize},
};
use rstest::{fixture, rstest};

use crate::contract::accounts::{
    Account,
    account::{
        AccountEntry,
        entry::{AccountV0, v1::AccountV1},
    },
};

#[fixture]
fn random_nonces(
    mut rng: impl Rng,
    #[default(10..1000)] size: impl RangeBounds<usize>,
) -> Vec<U256> {
    let future_deadline = Deadline::new(Utc::now().checked_add_days(Days::new(1)).unwrap());

    (0..range_to_random_size(&mut rng, size))
        .map(|_| {
            let expirable = rng.random();
            if expirable {
                ExpirableNonce::new(future_deadline, rng.random()).into()
            } else {
                rng.random()
            }
        })
        .collect()
}

#[rstest]
#[case::v0(PhantomData::<AccountV0>)]
#[case::v1(PhantomData::<AccountV1>)]
#[allow(clippy::used_underscore_binding)]
fn legacy_upgrade<T>(
    #[from(make_arbitrary)] data: AccountData,
    random_nonces: Vec<U256>,
    #[case] _marker: PhantomData<T>,
) where
    T: LegacyAccountBuilder,
    <T as LegacyAccountBuilder>::Account: BorshSerialize + BorshDeserialize,
{
    let legacy = data.make_legacy_account::<T>();
    let serialized_legacy = borsh::to_vec(&legacy).expect("unable to serialize legacy Account");
    // we need to drop it, so all collections from near-sdk flush to storage
    drop(legacy);

    let mut versioned: AccountEntry = borsh::from_slice(&serialized_legacy).unwrap();
    let account = versioned
        .lock()
        .expect("legacy accounts must be unlocked by default");
    data.assert_contained_in(account);

    // commit new nonces
    for nonce in &random_nonces {
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
    for &n in &random_nonces {
        assert!(account.is_nonce_used(n));
    }
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
    fn make_legacy_account<B: LegacyAccountBuilder>(&self) -> B::Account {
        let mut legacy = B::new(self.prefix.as_slice(), &self.account_id);

        self.public_keys
            .iter()
            .for_each(|&pk| assert!(B::add_public_key(&mut legacy, &self.account_id, pk)));

        if let Some(pk) = PublicKey::from_implicit_account_id(&self.account_id)
            .filter(|_| self.try_remove_implicit_public_key)
        {
            assert!(B::remove_public_key(&mut legacy, &self.account_id, &pk));
        }

        self.nonces
            .iter()
            .for_each(|&n| assert!(B::commit_nonce(&mut legacy, n).is_ok()));

        self.token_balances.iter().for_each(|(token_id, &amount)| {
            assert!(B::add_balance(&mut legacy, token_id.clone(), amount));
        });

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
    type Account;

    fn new(prefix: &[u8], account_id: &AccountId) -> Self::Account;
    fn add_public_key(account: &mut Self::Account, account_id: &AccountId, pk: PublicKey) -> bool;
    fn remove_public_key(
        account: &mut Self::Account,
        account_id: &AccountId,
        pk: &PublicKey,
    ) -> bool;
    fn commit_nonce(account: &mut Self::Account, nonce: U256) -> Result<()>;
    fn add_balance(account: &mut Self::Account, token_id: TokenId, amount: u128) -> bool;
}

// Added macro for builder implementation to reduce boilerplate
macro_rules! impl_legacy_account_builder {
    ($account_type:ty) => {
        impl LegacyAccountBuilder for $account_type {
            type Account = $account_type;

            fn new(prefix: &[u8], account_id: &AccountId) -> Self::Account {
                <$account_type>::new(prefix, account_id)
            }

            fn add_public_key(
                account: &mut Self::Account,
                account_id: &AccountId,
                pk: PublicKey,
            ) -> bool {
                account.add_public_key(account_id, pk)
            }

            fn remove_public_key(
                account: &mut Self::Account,
                account_id: &AccountId,
                pk: &PublicKey,
            ) -> bool {
                account.remove_public_key(account_id, pk)
            }

            fn commit_nonce(account: &mut Self::Account, nonce: U256) -> Result<()> {
                account.commit_nonce(nonce)
            }

            fn add_balance(account: &mut Self::Account, token_id: TokenId, amount: u128) -> bool {
                account.token_balances.add(token_id, amount).is_some()
            }
        }
    };
}

impl_legacy_account_builder!(AccountV0);
impl_legacy_account_builder!(AccountV1);
