use anyhow::Result;
use near_workspaces::Account;
use std::collections::HashMap;

use defuse::{
    contract::Role,
    core::{
        Deadline,
        intents::{DefuseIntents, Intent, account::AddPublicKey},
        token_id::{TokenId, nep141::Nep141TokenId},
    },
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
                state::{AccountData, PermanentState},
            },
            intents::ExecuteIntentsExt,
            state::FeesManagerExt,
        },
        poa::factory::PoAFactoryExt,
    },
    utils::{acl::AclExt, ft::FtExt, mt::MtExt},
};

pub trait StorageMigration {
    async fn generate_storage_data(&mut self);
    async fn verify_storage_consistency(&self);
}
macro_rules! execute_parallel {
    ($self:expr, $items:ident, $task:ident) => {{
        let state = $self.arbitrary_state.as_ref().unwrap();

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
        let state = PermanentState::generate().unwrap();
        self.arbitrary_state = Some(state);

        self.apply_fees().await.expect("Failed to apply fees");

        execute_parallel!(self, accounts, apply_account).expect("Failed to apply accounts");
        execute_parallel!(self, token_names, apply_token).expect("Failed to apply tokens");
        execute_parallel!(self, token_balances, apply_token_balance)
            .expect("Failed to apply token balances");
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
            let account_id = self.sandbox.get_subaccount_id(&data.name);

            let enabled = self
                .defuse
                .is_auth_by_predecessor_id_enabled(&account_id)
                .await
                .unwrap();

            assert_eq!(data.disable_auth_by_predecessor, !enabled);

            for pubkey in &data.public_keys {
                assert!(
                    self.defuse
                        .has_public_key(&account_id, pubkey)
                        .await
                        .unwrap()
                );
            }

            for nonce in &data.nonces {
                assert!(self.defuse.is_nonce_used(&account_id, nonce).await.unwrap());
            }
        }

        for (account_name, balance) in &state.token_balances {
            let account_id = self.sandbox.get_subaccount_id(&account_name);

            let tokens = balance
                .keys()
                .map(|t| {
                    TokenId::from(Nep141TokenId::new(Account::token_id(
                        t,
                        self.poa_factory.id(),
                    )))
                    .to_string()
                })
                .collect::<Vec<_>>();

            let balances = self
                .mt_contract_batch_balance_of(self.defuse.id(), &account_id, &tokens)
                .await
                .unwrap();

            // FIXME: looks like total shit
            let expected = balance.values().cloned().collect::<Vec<_>>();

            assert_eq!(balances, expected);
        }
    }
}

impl Env {
    async fn apply_token(&self, token_name: &str) -> Result<()> {
        let root = self.sandbox.root_account();

        let token = root
            .poa_factory_deploy_token(&self.poa_factory.id(), token_name, None)
            .await?;

        self.poa_factory
            .ft_storage_deposit_many(&token, &[root.id(), self.defuse.id()])
            .await?;

        root.poa_factory_ft_deposit(
            self.poa_factory.id(),
            token_name,
            root.id(),
            1_000_000_000,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    async fn apply_token_balance(&self, data: (&String, &HashMap<String, u128>)) -> Result<()> {
        let (account_name, balances) = data;
        let account_id = self.sandbox.get_subaccount_id(&account_name);

        balances
            .iter()
            .map(|(token, amount)| async {
                let token_id = Account::token_id(token, self.poa_factory.id());

                self.defuse_ft_deposit_to(&token_id, *amount, &account_id)
                    .await
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await?;

        Ok(())
    }

    async fn apply_fees(&self) -> Result<()> {
        let state = self.arbitrary_state.as_ref().unwrap();

        self.acl_grant_role(
            self.defuse.id(),
            Role::FeesManager,
            self.sandbox.root_account().id(),
        )
        .await?;

        self.set_fee(self.defuse.id(), state.fees.fee).await?;

        self.set_fee_collector(self.defuse.id(), &state.fees.fee_collector)
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

    async fn apply_account(&self, account: &AccountData) -> Result<()> {
        let acc = self.create_user(&account.name).await;

        if account.disable_auth_by_predecessor {
            acc.disable_auth_by_predecessor_id(self.defuse.id()).await?;
        }

        self.apply_public_keys(&acc, account).await?;

        self.apply_nonces(&acc, account).await?;

        Ok(())
    }
}
