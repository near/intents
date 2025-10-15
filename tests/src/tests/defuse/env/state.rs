use std::collections::{HashMap, HashSet};

use arbitrary::{Arbitrary, Unstructured};
use arbitrary_with::{ArbitraryAs, As};
use defuse::core::{
    Nonce, Salt,
    crypto::PublicKey,
    fees::{FeesConfig, Pips},
};
use defuse_near_utils::arbitrary::ArbitraryAccountId;
use defuse_randomness::Rng;
use near_sdk::AccountId;

use defuse::core::{intents::Intent, token_id::TokenId};
use near_workspaces::Account;

const MAX_ACCOUNTS: usize = 5;
const MAX_SALTS: usize = 5;

#[derive(Debug, Clone)]
pub struct AccountData {
    pub account: Account,

    pub public_keys: HashSet<PublicKey>,

    pub nonces: HashSet<Nonce>,

    pub token_balances: HashMap<TokenId, u128>,

    pub disable_auth_by_predecessor: bool,
}

// #[derive(Arbitrary, Debug, Clone, PartialEq, Eq)]
// pub struct AccountData {
//     #[arbitrary(with = generate_arbitrary_account)]
//     pub account: Account,

//     #[arbitrary(with = generate_arbitrary_pubkeys)]
//     pub public_keys: HashSet<PublicKey>,

//     #[arbitrary(with = arbitrary_default)]
//     pub nonces: HashSet<Nonce>,

//     #[arbitrary(with = arbitrary_default)]
//     pub token_balances: HashMap<TokenId, u128>,

//     pub disable_auth_by_predecessor: bool,
// }

// fn generate_arbitrary_account(u: &mut Unstructured) -> arbitrary::Result<HashSet<PublicKey>> {
//     let num_keys = u.int_in_range(0..=MAX_KEYS_PER_ACCOUNT)?;

//     (0..num_keys)
//         .map(|_| {
//             let key_bytes: [u8; 32] = u.arbitrary()?;
//             Ok(PublicKey::Ed25519(key_bytes))
//         })
//         .collect()
// }

// fn generate_arbitrary_pubkeys(u: &mut Unstructured) -> arbitrary::Result<HashSet<PublicKey>> {
//     let num_keys = u.int_in_range(0..=MAX_KEYS_PER_ACCOUNT)?;

//     (0..num_keys)
//         .map(|_| {
//             let key_bytes: [u8; 32] = u.arbitrary()?;
//             Ok(PublicKey::Ed25519(key_bytes))
//         })
//         .collect()
// }

// fn arbitrary_default<T: Default>(_u: &mut Unstructured) -> arbitrary::Result<T> {
//     Ok(T::default())
// }

/// Generates arbitrary but consistent state changes through intents
#[derive(Debug)]
pub struct PermanentState {
    pub accounts: Vec<AccountData>,
    pub fees: FeesConfig,
}

impl PermanentState {
    pub fn get_random_account(&self, rng: &mut impl Rng) -> &Account {
        let index = rng.random_range(0..self.accounts.len());
        &self.accounts[index].account
    }
    // pub fn generate(rng: &mut impl Rng, random_bytes: &[u8]) -> Self {
    //     let u = &mut Unstructured::new(&random_bytes);

    //     let num_accounts = rng.random_range(1..=MAX_ACCOUNTS);
    //     let accounts = (0..num_accounts)
    //         .map(|_| AccountData::arbitrary(u).unwrap())
    //         .collect::<Vec<_>>();

    //     let fee: u32 = rng.random_range(..=1000);
    //     let fee_collector = ArbitraryAccountId::arbitrary_as(u).unwrap();
    //     let fees = FeesConfig {
    //         fee: Pips::from_pips(fee).unwrap(),
    //         fee_collector,
    //     };

    //     let salts = (0..MAX_SALTS)
    //         .map(|_| {
    //             let salt = Salt::arbitrary(u).unwrap();
    //             let value = rng.random();
    //             (salt, value)
    //         })
    //         .collect();

    //     Self { accounts, fees }
    // }
}
