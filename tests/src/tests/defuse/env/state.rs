use std::collections::{HashMap, HashSet};

use arbitrary::{Arbitrary, Unstructured};
use defuse::core::{Nonce, crypto::PublicKey, token_id::nep141::Nep141TokenId};
use defuse_test_utils::random::Seed;
use itertools::Itertools;
use near_sdk::{
    AccountId,
    env::{keccak512, sha256},
};
use near_workspaces::Account;

use crate::{tests::defuse::env::generate_legacy_user_account_id, utils::ParentAccount};

const MAX_PUBLIC_KEYS: usize = 10;
const MAX_ACCOUNTS: usize = 5;
const MAX_NONCES: usize = 5;
const MAX_TOKENS: usize = 3;

const MIN_BALANCE_AMOUNT: u128 = 1_000;
const MAX_BALANCE_AMOUNT: u128 = 10_000;

#[derive(Arbitrary, Debug, Clone, PartialEq, Eq)]
pub struct AccountData {
    pub public_keys: HashSet<PublicKey>,
    pub nonces: HashSet<Nonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountWithTokens {
    pub data: AccountData,
    pub tokens: HashMap<Nep141TokenId, u128>,
}

/// Generates arbitrary but consistent state changes
#[derive(Debug)]
pub struct PersistentState {
    pub accounts: HashMap<AccountId, AccountWithTokens>,
}

impl PersistentState {
    pub fn generate(root: &Account, factory: &Account, seed: Seed) -> Self {
        let tokens = (0..MAX_TOKENS)
            .map(|token_id| {
                Nep141TokenId::new(factory.subaccount_id(&format!("test-token-{token_id}")))
            })
            .collect::<Vec<_>>();

        Self {
            accounts: Self::generate_accounts(root, tokens.as_slice(), seed),
        }
    }

    pub fn get_tokens(&self) -> Vec<Nep141TokenId> {
        self.accounts
            .iter()
            .flat_map(|(_, account)| account.tokens.keys().cloned())
            .unique()
            .sorted()
            .collect()
    }

    fn generate_accounts(
        prefix: &Account,
        tokens: &[Nep141TokenId],
        seed: Seed,
    ) -> HashMap<AccountId, AccountWithTokens> {
        (0..MAX_ACCOUNTS)
            .map(|idx| {
                let subaccount = generate_legacy_user_account_id(prefix, idx, seed)
                    .expect("Failed to generate account ID");

                let public_keys = (0..MAX_PUBLIC_KEYS)
                    .map(|pk_index| {
                        let pkey_source =
                            keccak512(format!("{subaccount}-public-key-{pk_index}").as_bytes());
                        let mut u = Unstructured::new(pkey_source.as_slice());
                        u.arbitrary().unwrap()
                    })
                    .collect();

                let nonces = (0..MAX_NONCES)
                    .map(|nonce_index| {
                        Unstructured::new(
                            sha256(format!("{subaccount}-nonce-{nonce_index}").as_bytes())
                                .as_slice(),
                        )
                        .arbitrary()
                        .unwrap()
                    })
                    .collect();

                #[allow(clippy::as_conversions)]
                let account_tokens = tokens
                    .iter()
                    .map(|token| (token.clone(), MIN_BALANCE_AMOUNT + (idx as u128 * 1000u128)))
                    .collect();
                (
                    subaccount,
                    AccountWithTokens {
                        data: AccountData {
                            public_keys,
                            nonces,
                        },
                        tokens: account_tokens,
                    },
                )
            })
            .collect()
    }
}
