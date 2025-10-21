use anyhow::Result;
use near_sdk::AccountId;
use near_workspaces::Account;
use std::collections::HashMap;

use defuse::core::{
    Deadline,
    intents::{DefuseIntents, Intent, account::AddPublicKey},
    token_id::{TokenId, nep141::Nep141TokenId},
};
use defuse_randomness::{Rng, make_true_rng};
use futures::stream::FuturesUnordered;
use futures::stream::TryStreamExt;

use crate::{
    tests::{
        defuse::{
            DefuseSigner, SigningStandard,
            accounts::AccountManagerExt,
            env::{
                Env,
                state::{AccountData, PersistentState},
            },
            intents::ExecuteIntentsExt,
        },
        poa::factory::PoAFactoryExt,
    },
    utils::{ParentAccount, ft::FtExt, mt::MtExt},
};

pub trait StorageMigration {
    async fn generate_storage_data(&mut self);
    async fn verify_storage_consistency(&self);
}

macro_rules! execute_tasks {
    ($self:expr, $items:ident, $task:ident) => {{
        let state = $self.persistent_state.as_ref().unwrap();

        state
            .$items
            .iter()
            .map(|item| $self.$task(item))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await
    }};
}

impl StorageMigration for Env {
    async fn generate_storage_data(&mut self) {
        let state =
            PersistentState::generate(self.sandbox.root_account(), self.poa_factory.as_account());

        self.persistent_state = Some(state);

        execute_tasks!(self, accounts, apply_account).expect("Failed to apply accounts");
        execute_tasks!(self, tokens, apply_token).expect("Failed to apply tokens");
        execute_tasks!(self, token_balances, apply_token_balance)
            .expect("Failed to apply token balances");
    }

    async fn verify_storage_consistency(&self) {
        let Some(state) = self.persistent_state.as_ref() else {
            println!("Nothing to verify - persistent state is not set");
            return;
        };

        self.verify_accounts_consistency(state).await;

        self.verify_token_balances_consistency(state).await;
    }
}

impl Env {
    async fn apply_token(&self, token_id: &Nep141TokenId) -> Result<()> {
        let root = self.sandbox.root_account();
        let token_name = self
            .poa_factory
            .subaccount_name(&token_id.clone().into_contract_id());

        let token = root
            .poa_factory_deploy_token(self.poa_factory.id(), &token_name, None)
            .await?;

        self.poa_factory
            .ft_storage_deposit_many(&token, &[root.id(), self.defuse.id()])
            .await?;

        root.poa_factory_ft_deposit(
            self.poa_factory.id(),
            &token_name,
            root.id(),
            1_000_000_000,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    async fn apply_token_balance(
        &self,
        data: (&AccountId, &HashMap<Nep141TokenId, u128>),
    ) -> Result<()> {
        let (account_id, balances) = data;

        balances
            .iter()
            .map(|(token, amount)| async {
                self.defuse_ft_deposit_to(&token.clone().into_contract_id(), *amount, account_id)
                    .await
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await?;

        Ok(())
    }

    async fn apply_public_keys(&self, acc: &Account, data: &AccountData) -> Result<()> {
        let intents = data
            .public_keys
            .iter()
            .map(|public_key| {
                Intent::AddPublicKey(AddPublicKey {
                    public_key: *public_key,
                })
            })
            .collect();

        self.defuse
            .execute_intents_without_simulation([acc.sign_defuse_message(
                SigningStandard::default(),
                self.defuse.id(),
                make_true_rng().random(),
                Deadline::MAX,
                DefuseIntents { intents },
            )])
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

        self.defuse
            .execute_intents_without_simulation(payload)
            .await?;

        Ok(())
    }

    async fn apply_account(&self, data: (&AccountId, &AccountData)) -> Result<Account> {
        let (account_id, account) = data;
        let acc = self
            .create_named_user(&self.sandbox.subaccount_name(account_id))
            .await?;

        self.apply_public_keys(&acc, account).await?;

        self.apply_nonces(&acc, account).await?;

        Ok(acc)
    }

    async fn verify_accounts_consistency(&self, state: &PersistentState) {
        for (account_id, data) in &state.accounts {
            for pubkey in &data.public_keys {
                let has_key = self
                    .defuse
                    .has_public_key(account_id, pubkey)
                    .await
                    .unwrap();

                assert!(has_key);
            }

            for nonce in &data.nonces {
                let is_used = self.defuse.is_nonce_used(account_id, nonce).await.unwrap();
                assert!(is_used);
            }
        }
    }

    async fn verify_token_balances_consistency(&self, state: &PersistentState) {
        for (account_id, expected_balances) in &state.token_balances {
            let tokens: Vec<String> = expected_balances
                .keys()
                .map(|token_id| TokenId::Nep141(token_id.clone()).to_string())
                .collect();

            let actual_balances = self
                .mt_contract_batch_balance_of(self.defuse.id(), account_id, &tokens)
                .await
                .unwrap();

            let expected_values: Vec<u128> = expected_balances.values().copied().collect();

            assert_eq!(actual_balances, expected_values,);
        }
    }
}
