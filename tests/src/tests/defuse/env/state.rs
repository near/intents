use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use defuse::core::{Nonce, crypto::PublicKey};
use defuse_randomness::{Rng, make_true_rng, seq::IteratorRandom};
use defuse_test_utils::random::random_bytes;

const MAX_PUBLIC_KEYS: usize = 10;
const MAX_ACCOUNTS: usize = 5;
const MAX_NONCES: usize = 5;
const MAX_TOKENS: usize = 3;

const MIN_BALANCE_AMOUNT: u128 = 1_000;
const MAX_BALANCE_AMOUNT: u128 = 10_000;

#[derive(Arbitrary, Debug, Clone, PartialEq, Eq)]
pub struct AccountData {
    #[arbitrary(with = generate_arbitrary_name)]
    pub name: String,

    #[arbitrary(with = generate_limited_arbitrary::<MAX_ACCOUNTS, PublicKey>)]
    pub public_keys: HashSet<PublicKey>,

    #[arbitrary(with = generate_limited_arbitrary::<MAX_NONCES, Nonce>)]
    pub nonces: HashSet<Nonce>,

    pub disable_auth_by_predecessor: bool,
}

impl Hash for AccountData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

fn generate_arbitrary_name(u: &mut Unstructured) -> arbitrary::Result<String> {
    Ok(format!("test_user_{}", u8::arbitrary(u)?))
}

/// Generates arbitrary but consistent state changes
#[derive(Debug)]
pub struct PermanentState {
    pub accounts: HashSet<AccountData>,
    // Using String for token names for simplicity - there are used only NEP-141 tokens in tests
    pub token_balances: HashMap<String, HashMap<String, u128>>,
    pub token_names: HashSet<String>,
}

impl PermanentState {
    pub fn generate() -> Result<Self> {
        let mut rng = make_true_rng();
        let random_bytes = random_bytes(50..1000, &mut rng);
        let u = &mut Unstructured::new(&random_bytes);

        let accounts = generate_limited_arbitrary::<MAX_ACCOUNTS, AccountData>(u)?;

        let token_count = rng.random_range(1..=MAX_TOKENS);
        let token_names = (1..=token_count)
            .map(|i| format!("test_token_{}", i))
            .collect::<HashSet<_>>();

        let token_balances = generate_balances(&mut rng, &accounts, &token_names);

        Ok(Self {
            accounts,
            token_names,
            token_balances,
        })
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

fn generate_balances(
    mut rng: &mut impl Rng,
    accounts: &HashSet<AccountData>,
    token_names: &HashSet<String>,
) -> HashMap<String, HashMap<String, u128>> {
    if token_names.is_empty() {
        return HashMap::new();
    }

    accounts
        .into_iter()
        .map(|account| {
            let num_tokens = rng.random_range(1..=token_names.len());

            let balances = token_names
                .iter()
                .choose_multiple(&mut rng, num_tokens)
                .into_iter()
                .map(|token| {
                    let amount = rng.random_range(MIN_BALANCE_AMOUNT..=MAX_BALANCE_AMOUNT);
                    (token.clone(), amount)
                })
                .collect();

            (account.name.clone(), balances)
        })
        .collect()
}
