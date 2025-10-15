use core::num;
use std::collections::{HashMap, HashSet};

use arbitrary::{Arbitrary, Unstructured};
use arbitrary_with::ArbitraryAs;
use defuse::core::{
    crypto::PublicKey,
    fees::{FeesConfig, Pips},
    intents::{
        Intent,
        account::AddPublicKey,
        token_diff::{TokenDeltas, TokenDiff},
    },
    payload::multi::MultiPayload,
    token_id::{TokenId, nep141::Nep141TokenId},
};
use defuse_near_utils::arbitrary::ArbitraryAccountId;
use defuse_randomness::{Rng, make_true_rng};
use defuse_test_utils::random::random_bytes;
use near_sdk::{AccountId, store::vec};
use tokio::task::JoinSet;

use crate::{
    tests::{
        defuse::{
            accounts::AccountManagerExt,
            env::{
                Env,
                state::{AccountData, PermanentState},
            },
            intents::ExecuteIntentsExt,
            state::FeesManagerExt,
        },
        poa::factory::PoAFactoryExt,
    },
    utils::{ft::FtExt, mt::MtExt},
};

const MAX_PUBKEYS_PER_ACCOUNT: usize = 3;
const MAX_TOKENS: usize = 3;
const MAX_SALTS: usize = 5;
const MAX_ACCOUNTS: usize = 5;
const MAX_TRADES: usize = 5;

const MIN_TRADE_AMOUNT: u128 = 1_000;
const MAX_TRADE_AMOUNT: u128 = 10_000;

pub trait StorageMigration {
    async fn apply_storage_data(&mut self);
    async fn verify_storage_consistency(&self);
}

impl StorageMigration for Env {
    async fn apply_storage_data(&mut self) {
        let mut rng = make_true_rng();
        let random_bytes = random_bytes(50..1000, &mut rng);

        let mut accounts = self.apply_accounts(&mut rng, &random_bytes).await;
        self.apply_trades(&mut rng, &mut accounts).await;
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
    // TODO: make it parallel!
    async fn generate_account_data(&self, rng: &mut impl Rng) -> Vec<AccountData> {
        let mut accounts = Vec::new();
        let num_accs = make_true_rng().random_range(2..=MAX_ACCOUNTS);

        for num in 0..=num_accs {
            let account = self.create_user(&format!("account_{}", num)).await;

            accounts.push(AccountData {
                account: account.clone(),
                disable_auth_by_predecessor: rng.random(),
                public_keys: HashSet::new(),
                nonces: HashSet::new(),
                token_balances: HashMap::new(),
            });
        }

        accounts
    }

    // async fn generate_tokens(&self, accounts: &[&AccountData]) -> Vec<TokenId> {
    //     let mut set = JoinSet::new();
    //     let num_tokens = make_true_rng().random_range(2..=MAX_TOKENS);
    //     let accounts = accounts
    //         .iter()
    //         .map(|a| a.account.id().clone())
    //         .collect::<Vec<_>>();

    //     for num in 0..=num_tokens {
    //         // TODO: reduce cloning
    //         let root = self.sandbox.root_account().clone();
    //         let poa = self.poa_factory.clone();
    //         let accounts = accounts.clone();

    //         set.spawn(async move {
    //             let token_id = root
    //                 .poa_factory_deploy_token(&poa.id(), &format!("ft_{}", num), None)
    //                 .await
    //                 .unwrap();

    //             poa.ft_storage_deposit_many(&token_id, &accounts)
    //                 .await
    //                 .unwrap();

    //             TokenId::from(Nep141TokenId::new(token_id))
    //         });
    //     }

    //     let mut tokens = vec![];
    //     while let Some(result) = set.join_next().await {
    //         tokens.push(result.unwrap());
    //     }

    //     tokens
    // }

    async fn generate_tokens(&self) -> Vec<AccountId> {
        let mut set = JoinSet::new();
        let num_tokens = make_true_rng().random_range(2..=MAX_TOKENS);

        for num in 0..=num_tokens {
            let root = self.sandbox.root_account().clone();
            let poa_id = self.poa_factory.id().clone();

            set.spawn(async move {
                root.poa_factory_deploy_token(&poa_id, &format!("ft_{}", num), None)
                    .await
                    .unwrap()
            });
        }

        let mut tokens = vec![];
        while let Some(result) = set.join_next().await {
            tokens.push(result.unwrap());
        }

        tokens
    }

    fn generate_public_key_changes(
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
        let mut accounts = self.generate_account_data(rng).await;

        // let payload: Vec<_> = accounts
        //     .iter_mut()
        //     .map(|account| {
        //         let intents = self.generate_public_key_changes(account, rng, random_bytes);
        //         self.sign_intents(&account.account, intents)
        //     })
        //     .collect();

        // self.defuse.execute_intents(payload).await.unwrap();

        accounts
    }

    async fn apply_trades(&self, rng: &mut impl Rng, accounts: &mut Vec<AccountData>) {
        let tokens = self.generate_tokens().await;
        let trades = self.generate_trades(rng, accounts, &tokens);

        self.deposit_for_trades(accounts).await;

        self.defuse.execute_intents(trades).await.unwrap();
    }

    // TODO: make it parallel!
    async fn deposit_for_trades(&self, accounts: &[AccountData]) {
        for account in accounts {
            for (token, balance) in &account.token_balances {
                let id = account.account.id();

                // TODO: move it to token creation
                self.poa_factory_ft_deposit(
                    self.poa_factory.id(),
                    &self.poa_ft_name(token),
                    self.sandbox.root_account().id(),
                    1_000_000_000,
                    None,
                    None,
                )
                .await
                .unwrap();

                self.poa_factory
                    .ft_storage_deposit(token, Some(id))
                    .await
                    .unwrap();

                self.defuse_ft_deposit_to(&token, *balance, id)
                    .await
                    .unwrap();
            }
        }
    }

    fn generate_trades(
        &self,
        rng: &mut impl Rng,
        accounts: &mut [AccountData],
        tokens: &[AccountId],
    ) -> Vec<MultiPayload> {
        if tokens.is_empty() {
            return vec![];
        }

        let num_trades = rng.random_range(1..=MAX_TRADES);
        let mut payload = Vec::with_capacity(num_trades * 2);

        for _ in 0..num_trades {
            let (ft1_ix, ft2_ix) = get_random_pair_indices(rng, tokens.len());
            let (acc1_ix, acc2_ix) = get_random_pair_indices(rng, accounts.len());

            let amount_in = rng.random_range(MIN_TRADE_AMOUNT..=MAX_TRADE_AMOUNT) as i128;
            let amount_out = rng.random_range(MIN_TRADE_AMOUNT..=MAX_TRADE_AMOUNT) as i128;

            let ft1 = &tokens[ft1_ix];
            let ft2 = &tokens[ft2_ix];

            let deltas1 = create_deltas(ft1, ft2, -amount_in, amount_out);
            let deltas2 = create_deltas(ft1, ft2, amount_in, -amount_out);

            // Process account 1
            {
                let acc = &mut accounts[acc1_ix];
                payload.push(self.sign_intents(
                    &acc.account,
                    vec![Intent::TokenDiff(TokenDiff {
                        diff: deltas1,
                        memo: None,
                        referral: None,
                    })],
                ));

                update_balance(&mut acc.token_balances, ft1, -amount_in);
                update_balance(&mut acc.token_balances, ft2, amount_out);
            }

            // Process account 2
            {
                let acc = &mut accounts[acc2_ix];
                payload.push(self.sign_intents(
                    &acc.account,
                    vec![Intent::TokenDiff(TokenDiff {
                        diff: deltas2,
                        memo: None,
                        referral: None,
                    })],
                ));

                update_balance(&mut acc.token_balances, ft1, amount_in);
                update_balance(&mut acc.token_balances, ft2, -amount_out);
            }
        }

        payload
    }
}

fn get_random_pair_indices(rng: &mut impl Rng, len: usize) -> (usize, usize) {
    debug_assert!(len >= 2);

    let idx1 = rng.random_range(0..len);
    let idx2 = loop {
        let idx = rng.random_range(0..len);
        if idx != idx1 {
            break idx;
        }
    };

    (idx1, idx2)
}

fn create_deltas(
    token1: &AccountId,
    token2: &AccountId,
    delta1: i128,
    delta2: i128,
) -> TokenDeltas {
    let token1 = TokenId::from(Nep141TokenId::new(token1.clone()));
    let token2 = TokenId::from(Nep141TokenId::new(token2.clone()));

    TokenDeltas::default()
        .with_apply_deltas([(token1, delta1), (token2, delta2)])
        .expect("Failed to create deltas")
}

fn update_balance(balances: &mut HashMap<AccountId, u128>, token: &AccountId, delta: i128) {
    let entry = balances.entry(token.clone()).or_insert(0);

    *entry = entry.saturating_add(delta as u128);
    // if delta < 0 {
    //     *entry = entry.saturating_sub((-delta) as u128);
    // } else {
    //     *entry = entry.saturating_add(delta as u128);
    // }
}
