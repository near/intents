use std::collections::{HashMap, HashSet};

use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use arbitrary_with::{ArbitraryAs, As};
use defuse::core::{
    Nonce,
    crypto::PublicKey,
    fees::{FeesConfig, Pips},
};
use defuse_near_utils::arbitrary::ArbitraryAccountId;
use defuse_randomness::{Rng, make_true_rng, seq::IndexedRandom};
use defuse_test_utils::random::random_bytes;
use near_sdk::AccountId;

use defuse::core::token_id::TokenId;

const MAX_ACCOUNTS: usize = 5;
const MAX_NONCES: usize = 5;
const MAX_TOKENS: usize = 3;

const MIN_BALANCE_AMOUNT: u128 = 1_000;
const MAX_BALANCE_AMOUNT: u128 = 10_000;

#[derive(Arbitrary, Debug, Clone)]
pub struct AccountData {
    #[arbitrary(with = As::<ArbitraryAccountId>::arbitrary)]
    pub account_id: AccountId,

    // #[arbitrary(with = As::<LimitLen<MAX_ACCOUNTS, _>>::arbitrary)]
    #[arbitrary(with = generate_limited_arbitrary::<MAX_ACCOUNTS, _, _>)]
    pub public_keys: HashSet<PublicKey>,

    // #[arbitrary(with = As::<LimitLen<MAX_NONCES, HashSet<Nonce>>>::arbitrary)]
    #[arbitrary(with = generate_limited_arbitrary::<MAX_NONCES, _, _>)]
    pub nonces: HashSet<Nonce>,

    pub disable_auth_by_predecessor: bool,
}

/// Generates arbitrary but consistent state changes
#[derive(Debug)]
pub struct PermanentState {
    pub accounts: Vec<AccountData>,
    pub fees: FeesConfig,
    pub token_balances: HashMap<AccountId, HashMap<TokenId, u128>>,
    pub tokens: Vec<TokenId>,
}

impl PermanentState {
    pub fn get_random_account(&self, rng: &mut impl Rng) -> &AccountId {
        let index = rng.random_range(0..self.accounts.len());
        &self.accounts[index].account_id
    }

    pub fn generate() -> Result<Self> {
        let mut rng = make_true_rng();
        let random_bytes = random_bytes(50..1000, &mut rng);
        let u = &mut Unstructured::new(&random_bytes);

        let accounts = generate_limited_arbitrary::<MAX_ACCOUNTS, Vec<AccountData>, _>(u)?;

        let fee: u32 = rng.random_range(..=Pips::MAX.as_pips());
        let fee_collector = ArbitraryAccountId::arbitrary_as(u).unwrap();
        let fees = FeesConfig {
            fee: Pips::from_pips(fee).unwrap(),
            fee_collector,
        };

        // TODO: generate_only_fts
        let tokens = generate_limited_arbitrary::<MAX_TOKENS, Vec<TokenId>, _>(u)?;

        let token_balances = generate_balances(&mut rng, &accounts, &tokens);

        Ok(Self {
            accounts,
            fees,
            tokens,
            token_balances,
        })
    }
}

fn generate_limited_arbitrary<const MAX: usize, T, A>(u: &mut Unstructured) -> arbitrary::Result<T>
where
    A: for<'a> Arbitrary<'a>,
    T: FromIterator<A>,
{
    let len = u.int_in_range(0..=MAX).unwrap_or(0);

    Ok((0..len).filter_map(|_| A::arbitrary(u).ok()).collect::<T>())
}

fn generate_balances(
    mut rng: &mut impl Rng,
    accounts: &[AccountData],
    tokens: &[TokenId],
) -> HashMap<AccountId, HashMap<TokenId, u128>> {
    if tokens.is_empty() {
        return HashMap::new();
    }

    accounts
        .into_iter()
        .map(|account| {
            let num_tokens = rng.random_range(1..=tokens.len());

            let balances = tokens
                .choose_multiple(&mut rng, num_tokens)
                .map(|token| {
                    let amount = rng.random_range(MIN_BALANCE_AMOUNT..=MAX_BALANCE_AMOUNT);
                    (token.clone(), amount)
                })
                .collect();

            (account.account_id.clone(), balances)
        })
        .collect()
}
