use anyhow::Result;
use near_workspaces::Account;
use std::{
    collections::{HashMap, HashSet},
    os::macos::raw::stat,
};

use arbitrary::{Arbitrary, Unstructured};
use arbitrary_with::ArbitraryAs;
use defuse::core::{
    Deadline,
    crypto::PublicKey,
    fees::{FeesConfig, Pips},
    intents::{
        DefuseIntents, Intent,
        account::AddPublicKey,
        token_diff::{TokenDeltas, TokenDiff},
    },
    payload::multi::MultiPayload,
    token_id::{TokenId, nep141::Nep141TokenId},
};
use defuse_near_utils::arbitrary::ArbitraryAccountId;
use defuse_randomness::{Rng, make_true_rng};
use defuse_test_utils::random::random_bytes;
use futures::stream;
use futures::stream::TryStreamExt;
use futures::{StreamExt, stream::FuturesUnordered};
use near_sdk::AccountId;
use std::future::Future;
use tokio::task::JoinSet;

use crate::{
    tests::{
        defuse::{
            DefuseSigner, SigningStandard,
            accounts::AccountManagerExt,
            env::{
                Env,
                state::{self, AccountData, PermanentState},
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
    async fn generate_storage_data(&mut self);
    async fn verify_storage_consistency(&self);
}
macro_rules! execute_parallel {
    ($self:expr, $items:expr, $task:ident) => {{
        $items
            .iter()
            .map(|item| $self.$task(item))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await
    }};
}

impl StorageMigration for Env {
    async fn generate_storage_data(&mut self) {
        let state = PermanentState::generate().unwrap();

        // pub accounts: Vec<AccountData>,

        execute_parallel!(self, &state.tokens, apply_token).expect("Failed to apply tokens");
        execute_parallel!(self, &state.tokens, apply_token_balance)
            .expect("Failed to apply token balances");

        self.apply_fees().await.expect("Failed to apply fees");

        self.apply_accounts().await;

        self.arbitrary_state = Some(state);
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
    async fn apply_token(&self, token: &TokenId) -> Result<()> {
        let root = self.sandbox.root_account();

        let token = root
            .poa_factory_deploy_token(&self.poa_factory.id(), &format!("ft_{}", 123), None)
            .await?;

        // TODO:  Create with futures
        self.poa_factory
            .ft_storage_deposit_many(&token, &[root.id(), self.defuse.id()])
            .await?;

        root.poa_factory_ft_deposit(
            self.poa_factory.id(),
            &self.poa_ft_name(&token),
            root.id(),
            1_000_000_000,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    async fn apply_token_balance(&self, data: (AccountId, HashMap<AccountId, u128>)) -> Result<()> {
        let (account_id, balances) = data;

        balances
            .iter()
            .map(|(token, amount)| async {
                self.defuse_ft_deposit_to(token, *amount, &account_id).await
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await?;

        Ok(())
    }

    async fn apply_fees(&self) -> Result<()> {
        let state = self.arbitrary_state.as_ref().unwrap();

        self.defuse
            .set_fee(self.defuse.id(), state.fees.fee)
            .await?;

        self.defuse
            .set_fee_collector(self.defuse.id(), &state.fees.fee_collector)
            .await?;

        Ok(())
    }

    async fn apply_public_keys(&self, acc: &Account, data: &AccountData) -> Result<()> {
        let intents = data
            .public_keys
            .iter()
            .map(|public_key| {
                Intent::AddPublicKey(AddPublicKey {
                    public_key: public_key.clone(),
                })
            })
            .collect();

        self.defuse
            .execute_intents([self.sign_intents(&acc, intents)])
            .await?;

        Ok(())
    }

    async fn apply_nonces(&self, acc: &Account, data: &AccountData) -> Result<()> {
        let payload = data
            .nonces
            .iter()
            .map(|nonce| {
                acc.sign_defuse_message(
                    SigningStandard::default(),
                    self.defuse.id(),
                    *nonce,
                    Deadline::MAX,
                    DefuseIntents { intents: vec![] },
                )
            })
            .collect::<Vec<_>>();

        self.defuse.execute_intents(payload).await?;

        Ok(())
    }

    async fn apply_accounts(&self) -> Result<()> {
        let state = self.arbitrary_state.as_ref().unwrap();

        // state.accounts.iter().map(|data| {

        // });

        for account in &state.accounts {
            // TODO: fix this
            let acc = self.create_user("123").await;

            if account.disable_auth_by_predecessor {
                self.defuse
                    .disable_auth_by_predecessor_id(&acc.id())
                    .await?;
            }

            self.apply_public_keys(&acc, account).await?;
            self.apply_nonces(&acc, account).await?;
        }

        Ok(())
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
