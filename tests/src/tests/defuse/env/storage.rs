use std::collections::{BTreeMap, HashMap, HashSet};

use arbitrary::{Arbitrary, Unstructured};
use arbitrary_with::ArbitraryAs;
use defuse::{
    core::{
        Deadline, Salt,
        amounts::Amounts,
        crypto::PublicKey,
        fees::{FeesConfig, Pips},
        intents::{
            DefuseIntents, Intent,
            account::{self, AddPublicKey},
            token_diff::{TokenDeltas, TokenDiff},
        },
        payload::multi::MultiPayload,
        token_id::TokenId,
    },
    intents::Intents,
};
use defuse_near_utils::arbitrary::ArbitraryAccountId;
use defuse_randomness::{Rng, make_true_rng, seq::IteratorRandom};
use defuse_test_utils::random::random_bytes;
use near_sdk::{AccountId, store::vec};
use near_workspaces::Account;

use crate::{
    tests::defuse::{
        DefuseSigner, SigningStandard,
        accounts::AccountManagerExt,
        env::{
            Env,
            state::{AccountData, PermanentState},
        },
        intents::ExecuteIntentsExt,
        state::{FeesManagerExt, SaltManagerExt},
    },
    utils::mt::MtExt,
};

const MAX_PUBKEYS_PER_ACCOUNT: usize = 3;
const MAX_TOKENS: usize = 5;
const MAX_SALTS: usize = 5;
const MAX_ACCOUNTS: usize = 5;
const MAX_TRADES: usize = 5;

const MIN_TRADE_AMOUNT: u128 = 1_000;
const MAX_TRADE_AMOUNT: u128 = 1_000_000;

pub trait StorageMigration {
    async fn apply_storage_data(&mut self);
    async fn verify_storage_consistency(&self);
}

impl StorageMigration for Env {
    async fn apply_storage_data(&mut self) {
        let mut rng = make_true_rng();
        let random_bytes = random_bytes(50..1000, &mut rng);

        let accounts = self.apply_accounts(&mut rng, &random_bytes).await;
        let fees = self.apply_fees(&mut rng, &random_bytes).await;

        self.arbitrary_state = Some(PermanentState { accounts, fees });
    }

    async fn verify_storage_consistency(&self) {
        let Some(state) = self.arbitrary_state.as_ref() else {
            println!("Nothing to verify - persistent state is not set");
            return;
        };

        let fee = self.defuse.fee(&self.defuse.id()).await.unwrap();
        assert_eq!(fee, state.fees.fee);

        let fee_collector = self.defuse.fee_collector(&self.defuse.id()).await.unwrap();
        assert_eq!(fee_collector, state.fees.fee_collector);

        for data in &state.accounts {
            let enabled = self
                .defuse
                .is_auth_by_predecessor_id_enabled(&data.account.id())
                .await
                .unwrap();

            assert_eq!(data.disable_auth_by_predecessor, !enabled);

            for pubkey in &data.public_keys {
                assert!(
                    self.defuse
                        .has_public_key(&data.account.id(), pubkey)
                        .await
                        .unwrap()
                );
            }

            for nonce in &data.nonces {
                assert!(
                    self.defuse
                        .is_nonce_used(&data.account.id(), nonce)
                        .await
                        .unwrap()
                );
            }

            let tokens = &data
                .token_balances
                .keys()
                .map(|t| t.to_string())
                .collect::<Vec<String>>();

            let balances = self
                .defuse
                .mt_batch_balance_of(&data.account.id(), tokens)
                .await
                .unwrap();

            for (pos, (_, amount)) in data.token_balances.iter().enumerate() {
                let balance = balances.get(pos).unwrap();
                assert_eq!(*balance, *amount);
            }
        }
    }
}

impl Env {
    async fn generate_accounts(&self) -> Vec<Account> {
        let mut accounts = Vec::new();
        for num in 2..MAX_ACCOUNTS {
            let account = self.create_user(&format!("account_{}", num)).await;

            accounts.push(account);
        }

        accounts
    }

    async fn generate_tokens(&self) -> Vec<AccountId> {
        let mut tokens = Vec::new();
        for num in 2..MAX_TOKENS {
            let token = self.create_token(&format!("ft_{}", num)).await;

            tokens.push(token);
        }

        tokens
    }

    async fn apply_fees(&self, rng: &mut impl Rng, random_bytes: &[u8]) -> FeesConfig {
        let fee = Pips::from_pips(rng.random_range(0..=Pips::MAX.as_pips())).unwrap();
        let u = &mut Unstructured::new(random_bytes);
        let fee_collector = ArbitraryAccountId::arbitrary_as(u).unwrap();

        self.defuse.set_fee(self.defuse.id(), fee).await.unwrap();
        self.defuse
            .set_fee_collector(self.defuse.id(), &fee_collector)
            .await
            .unwrap();

        return FeesConfig { fee, fee_collector };
    }

    async fn apply_accounts(&self, rng: &mut impl Rng, random_bytes: &[u8]) -> Vec<AccountData> {
        let accounts = self.generate_accounts().await;
        let tokens = self.generate_tokens().await;

        let mut accounts_data = vec![];

        for account in accounts {
            let mut data = AccountData {
                account: account.clone(),
                disable_auth_by_predecessor: rng.random(),
                public_keys: HashSet::new(),
                nonces: HashSet::new(),
                token_balances: HashMap::new(),
            };

            let pubkey_intents = self
                .generate_public_key_changes(&mut data, rng, random_bytes)
                .await;

            let signed = sign_intents(self.defuse.id(), &account, pubkey_intents);
            self.defuse.execute_intents([signed]).await.unwrap();

            accounts_data.push(data);
        }

        accounts_data
    }

    async fn generate_public_key_changes(
        &self,
        acc: &mut AccountData,
        rng: &mut impl Rng,
        random_bytes: &[u8],
    ) -> Vec<Intent> {
        let u = &mut Unstructured::new(random_bytes);
        let num_operations = rng.random_range(0..=MAX_PUBKEYS_PER_ACCOUNT);

        (0..num_operations)
            .map(|_| {
                let public_key = PublicKey::arbitrary(u).unwrap();

                acc.public_keys.insert(public_key.clone());

                Intent::AddPublicKey(AddPublicKey {
                    public_key: public_key.clone(),
                })
            })
            .collect()
    }

    fn generate_trades(
        &mut self,
        mut rng: &mut impl Rng,
        accounts: &mut [AccountData],
        tokens: &[TokenId],
    ) -> Vec<MultiPayload> {
        let mut payload = vec![];
        if tokens.is_empty() {
            return payload;
        }

        let num_trades = rng.random_range(1..=MAX_TRADES);

        for _ in 0..num_trades {
            // Pick two different tokens for the trade
            let (ft1, ft2) = get_random_pair(&mut rng, tokens);

            // Pick two different accounts for the trade
            let (account1, account2) = get_random_pair(&mut rng, accounts);

            let amount_in = rng.random_range(MIN_TRADE_AMOUNT..=MAX_TRADE_AMOUNT) as i128;
            let amount_out = rng.random_range(MIN_TRADE_AMOUNT..=MAX_TRADE_AMOUNT) as i128;

            let deltas1 = TokenDeltas::default()
                .with_apply_deltas([(ft1.clone(), -amount_in), (ft2.clone(), amount_out)])
                .unwrap();
            let deltas2 = TokenDeltas::default()
                .with_apply_deltas([(ft1.clone(), amount_in), (ft2.clone(), -amount_out)])
                .unwrap();

            payload.push(self.sign_intents(
                &account1.account,
                vec![Intent::TokenDiff(TokenDiff {
                    diff: deltas1,
                    memo: None,
                    referral: None,
                })],
            ));

            payload.push(self.sign_intents(
                &account2.account,
                vec![Intent::TokenDiff(TokenDiff {
                    diff: deltas2,
                    memo: None,
                    referral: None,
                })],
            ));

            account1
                .token_balances
                .entry(ft1.clone())
                .and_modify(|b| *b = *b - amount_in as u128);
            account1
                .token_balances
                .entry(ft2.clone())
                .and_modify(|b| *b = *b + amount_out as u128)
                .or_insert(amount_out as u128);

            account2
                .token_balances
                .entry(ft1.clone())
                .and_modify(|b| *b = *b + amount_in as u128)
                .or_insert(amount_in as u128);
            account2
                .token_balances
                .entry(ft2.clone())
                .and_modify(|b| *b = *b - amount_out as u128);
        }

        payload
    }
}

fn get_random_pair<'a, T>(mut rng: &mut impl Rng, data: &'a mut [T]) -> (&'a mut T, &'a mut T) {
    let mut iter = (0..data.len()).choose_multiple(&mut rng, 2).into_iter();

    (
        data.get_mut(iter.next().unwrap()).unwrap(),
        data.get_mut(iter.next().unwrap()).unwrap(),
    )
}
