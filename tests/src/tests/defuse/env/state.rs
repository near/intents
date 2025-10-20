use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use defuse::core::{Nonce, crypto::PublicKey, token_id::nep141::Nep141TokenId};
use defuse_near_utils::arbitrary::ArbitraryNamedAccountId;
use defuse_randomness::{Rng, make_true_rng, seq::IteratorRandom};
use defuse_test_utils::random::random_bytes;
use near_sdk::AccountId;
use near_workspaces::Account;

const MAX_PUBLIC_KEYS: usize = 10;
const MAX_ACCOUNTS: usize = 5;
const MAX_NONCES: usize = 5;
const MAX_TOKENS: usize = 3;

const MIN_BALANCE_AMOUNT: u128 = 1_000;
const MAX_BALANCE_AMOUNT: u128 = 10_000;

#[derive(Arbitrary, Debug, Clone, PartialEq, Eq)]
pub struct AccountData {
    #[arbitrary(with = generate_limited_arbitrary::<MAX_PUBLIC_KEYS, PublicKey>)]
    pub public_keys: HashSet<PublicKey>,

    #[arbitrary(with = generate_limited_arbitrary::<MAX_NONCES, Nonce>)]
    pub nonces: HashSet<Nonce>,
}

/// Generates arbitrary but consistent state changes
#[derive(Debug)]
pub struct PersistentState {
    pub accounts: HashMap<AccountId, AccountData>,
    pub token_balances: HashMap<AccountId, HashMap<Nep141TokenId, u128>>,
    pub tokens: HashSet<Nep141TokenId>,
}

impl PersistentState {
    pub fn generate(root: &Account, factory: &Account) -> Result<Self> {
        let mut rng = make_true_rng();
        let random_bytes = random_bytes(100..1000, &mut rng);
        let u = &mut Unstructured::new(&random_bytes);

        let accounts = Self::generate_accounts(u, root);
        let tokens = Self::generate_tokens(u, factory);
        let token_balances = Self::generate_balances(&mut rng, &accounts, &tokens);

        Ok(Self {
            accounts,
            tokens,
            token_balances,
        })
    }

    fn generate_accounts(u: &mut Unstructured, root: &Account) -> HashMap<AccountId, AccountData> {
        let number = u.int_in_range(1..=MAX_ACCOUNTS).unwrap();

        (0..number)
            .map(|_| {
                (
                    ArbitraryNamedAccountId::arbitrary_subaccount(u, Some(root.id())).unwrap(),
                    AccountData::arbitrary(u).unwrap(),
                )
            })
            .collect()
    }

    fn generate_tokens(u: &mut Unstructured, factory: &Account) -> HashSet<Nep141TokenId> {
        let number = u.int_in_range(1..=MAX_TOKENS).unwrap();

        (0..number)
            .map(|_| {
                Nep141TokenId::new(
                    ArbitraryNamedAccountId::arbitrary_subaccount(u, Some(factory.id())).unwrap(),
                )
            })
            .collect()
    }

    fn generate_balances(
        mut rng: &mut impl Rng,
        accounts: &HashMap<AccountId, AccountData>,
        tokens: &HashSet<Nep141TokenId>,
    ) -> HashMap<AccountId, HashMap<Nep141TokenId, u128>> {
        accounts
            .into_iter()
            .map(|(account_id, _)| {
                let num_tokens = rng.random_range(1..=tokens.len());

                let balances = tokens
                    .iter()
                    .choose_multiple(&mut rng, num_tokens)
                    .into_iter()
                    .map(|token| {
                        let amount = rng.random_range(MIN_BALANCE_AMOUNT..=MAX_BALANCE_AMOUNT);
                        (token.clone(), amount)
                    })
                    .collect();

                (account_id.clone(), balances)
            })
            .collect()
    }
}

fn generate_limited_arbitrary<const MAX: usize, T>(
    u: &mut Unstructured,
) -> arbitrary::Result<HashSet<T>>
where
    T: for<'a> Arbitrary<'a>,
    T: Eq + Hash,
{
    let len = u.int_in_range(2..=MAX).unwrap_or(0);

    Ok((0..len)
        .filter_map(|_| T::arbitrary(u).ok())
        .collect::<HashSet<T>>())
}
