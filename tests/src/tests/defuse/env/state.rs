use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use arbitrary::{Arbitrary, Unstructured};
use defuse::core::{Nonce, crypto::PublicKey, token_id::nep141::Nep141TokenId};
use defuse_near_utils::arbitrary::ArbitraryNamedAccountId;
use defuse_randomness::{RngCore, make_true_rng};
use defuse_test_utils::random::Seed;
use itertools::Itertools;
use near_sdk::AccountId;
use near_workspaces::Account;

use anyhow::Result;

use crate::tests::defuse::env::generate_determined_user_account_id;

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

    // NOTE: Generating legacy nonces for compatibility testing
    #[arbitrary(with = generate_limited_arbitrary::<MAX_NONCES, Nonce>)]
    pub nonces: HashSet<Nonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountWithTokens {
    pub data: AccountData,
    pub tokens: HashMap<Nep141TokenId, u128>,
}

impl AccountWithTokens {
    pub fn generate(
        tokens: impl IntoIterator<Item = Nep141TokenId>,
        u: &mut Unstructured,
    ) -> Result<Self> {
        let data = AccountData::arbitrary(u)?;
        let tokens = tokens.into_iter().collect::<Vec<_>>();

        let selected_token_amount = u.int_in_range(1..=tokens.len())?;

        let tokens = (0..selected_token_amount)
            .map(|_| {
                let ix = u.int_in_range(0..=tokens.len() - 1)?;
                let token = tokens[ix].clone();
                let amount = u.int_in_range(MIN_BALANCE_AMOUNT..=MAX_BALANCE_AMOUNT)?;

                anyhow::Ok((token, amount))
            })
            .collect::<Result<_, _>>()?;

        Ok(Self { data, tokens })
    }
}

/// Generates arbitrary but consistent state changes
#[derive(Debug)]
pub struct PersistentState {
    pub accounts: HashMap<AccountId, AccountWithTokens>,
}

impl PersistentState {
    pub fn generate(root: &Account, factory: &Account, seed: Seed) -> Result<Self> {
        let mut rng = make_true_rng();
        let mut random_bytes = [0u8; 1024];
        rng.fill_bytes(&mut random_bytes);

        let u = &mut Unstructured::new(&random_bytes);

        let tokens = Self::generate_tokens(u, factory)?;
        let accounts = Self::generate_accounts(u, root, tokens, seed)?;

        Ok(Self { accounts })
    }

    pub fn get_tokens(&self) -> Vec<Nep141TokenId> {
        self.accounts
            .iter()
            .flat_map(|(_, account)| account.tokens.keys().map(|t| t.clone()))
            .unique()
            .sorted()
            .collect()
    }

    fn generate_accounts(
        u: &mut Unstructured,
        root: &Account,
        tokens: impl IntoIterator<Item = Nep141TokenId>,
        seed: Seed,
    ) -> Result<HashMap<AccountId, AccountWithTokens>> {
        let number = u.int_in_range(1..=MAX_ACCOUNTS)?;
        let tokens = tokens.into_iter().collect::<Vec<_>>();

        (0..number)
            .map(|index| {
                let account_id = generate_determined_user_account_id(root.id(), seed, index)?;
                let account = AccountWithTokens::generate(tokens.clone(), u)?;
                Ok((account_id, account))
            })
            .collect()
    }

    fn generate_tokens(u: &mut Unstructured, factory: &Account) -> Result<HashSet<Nep141TokenId>> {
        let number = u.int_in_range(1..=MAX_TOKENS)?;

        (0..number)
            .map(|_| {
                let account_id =
                    ArbitraryNamedAccountId::arbitrary_subaccount(u, Some(factory.id()))?;

                Ok(Nep141TokenId::new(account_id))
            })
            .collect()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn generate_limited_arbitrary<const MAX: usize, T>(
    u: &mut Unstructured,
) -> arbitrary::Result<HashSet<T>>
where
    T: for<'a> Arbitrary<'a> + Eq + Hash,
{
    let len = u.int_in_range(2..=MAX).unwrap_or(0);

    Ok((0..len)
        .filter_map(|_| T::arbitrary(u).ok())
        .collect::<HashSet<T>>())
}
