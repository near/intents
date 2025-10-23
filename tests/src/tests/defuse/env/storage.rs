use anyhow::Result;
use near_sdk::AccountId;
use near_workspaces::Account;
use std::collections::{HashMap, HashSet};

use defuse::core::{
    Deadline, Nonce,
    crypto::PublicKey,
    intents::{DefuseIntents, Intent, account::AddPublicKey},
    token_id::{TokenId, nep141::Nep141TokenId},
};
use futures::future::try_join_all;

use crate::{
    tests::{
        defuse::{
            DefusePayloadBuilder, DefuseSigner, SigningStandard,
            accounts::AccountManagerExt,
            env::{
                Env,
                state::{AccountData, PersistentState},
            },
            intents::ExecuteIntentsExt,
        },
        poa::factory::PoAFactoryExt,
    },
    utils::{ParentAccount, mt::MtExt},
};

impl Env {
    pub async fn generate_storage_data(&mut self) {
        let state =
            PersistentState::generate(self.sandbox.root_account(), self.poa_factory.as_account());

        self.persistent_state = Some(state);

        futures::join!(self.apply_tokens(), self.apply_accounts());

        self.apply_token_balances().await;
    }

    pub async fn verify_storage_consistency(&self) {
        futures::join!(
            self.verify_accounts_consistency(),
            self.verify_token_balances_consistency()
        );
    }

    pub async fn apply_tokens(&self) {
        let state = self.state();

        try_join_all(
            state
                .tokens
                .iter()
                .map(|token_id| self.apply_token(token_id)),
        )
        .await
        .expect("Failed to apply tokens");
    }

    async fn apply_accounts(&self) {
        let state = self.state();

        try_join_all(state.accounts.iter().map(|data| self.apply_account(data)))
            .await
            .expect("Failed to apply accounts");
    }

    async fn apply_token_balances(&self) {
        let state = self.state();

        // NOTE: should be sequential to match contract token ordering
        for (token_id, balances) in &state.token_balances {
            let token_id = token_id.clone().into_contract_id();

            try_join_all(balances.iter().map(|(account_id, amount)| {
                self.defuse_ft_deposit_to(&token_id, *amount, account_id)
            }))
            .await
            .expect("Failed to apply token balances");
        }
    }

    async fn apply_token(&self, token_id: &Nep141TokenId) -> Result<()> {
        let root = self.sandbox.root_account();
        let token_name = self
            .poa_factory
            .subaccount_name(&token_id.clone().into_contract_id());

        let token = root
            .poa_factory_deploy_token(self.poa_factory.id(), &token_name, None)
            .await?;

        self.ft_storage_deposit_for_accounts(&token, vec![root.id(), self.defuse.id()])
            .await?;

        self.ft_deposit_to_root(&token).await?;

        Ok(())
    }

    async fn apply_public_keys(&self, acc: &Account, data: &AccountData) -> Result<()> {
        if data.public_keys.is_empty() {
            return Ok(());
        }

        let payload = acc
            .create_defuse_payload(
                &self.defuse.id(),
                data.public_keys
                    .iter()
                    .copied()
                    .map(|public_key| Intent::AddPublicKey(AddPublicKey { public_key })),
            )
            .await?;

        self.defuse
            .execute_intents_without_simulation([payload])
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

        futures::try_join!(
            self.apply_public_keys(&acc, account),
            self.apply_nonces(&acc, account)
        )?;

        Ok(acc)
    }

    async fn verify_accounts_consistency(&self) {
        let state = self.state();

        for (account_id, data) in &state.accounts {
            futures::join!(
                self.verify_public_keys(account_id, &data.public_keys),
                self.verify_nonces(account_id, &data.nonces)
            );
        }
    }

    async fn verify_public_keys(&self, account_id: &AccountId, public_keys: &HashSet<PublicKey>) {
        let res = try_join_all(
            public_keys
                .iter()
                .map(|p| self.defuse.has_public_key(account_id, p)),
        )
        .await
        .expect("Failed to verify publick keys");

        assert!(res.into_iter().all(|has_key| has_key),);
    }

    async fn verify_nonces(&self, account_id: &AccountId, nonces: &HashSet<Nonce>) {
        let res = try_join_all(
            nonces
                .iter()
                .map(|n| self.defuse.is_nonce_used(account_id, n)),
        )
        .await
        .expect("Failed to verify nonces");

        assert!(res.into_iter().all(|is_used| is_used));
    }

    async fn verify_token_balances_consistency(&self) {
        let state = self.state();

        try_join_all(
            state
                .token_balances
                .iter()
                .map(|(token_id, expected_balances)| {
                    self.verify_token_balance(token_id, expected_balances)
                }),
        )
        .await
        .expect("Failed to verify token balances");
    }

    async fn verify_token_balance(
        &self,
        token_id: &Nep141TokenId,
        expected_balances: &HashMap<AccountId, u128>,
    ) -> Result<()> {
        try_join_all(
            expected_balances
                .iter()
                .map(|(account_id, amount)| self.check_balance(account_id, token_id, *amount)),
        )
        .await?;

        Ok(())
    }

    async fn check_balance(
        &self,
        account_id: &AccountId,
        token_id: &Nep141TokenId,
        amount: u128,
    ) -> Result<()> {
        let token_id = TokenId::Nep141(token_id.clone()).to_string();
        let received = self
            .mt_contract_balance_of(self.defuse.id(), account_id, &token_id)
            .await?;

        if received != amount {
            anyhow::bail!("Token balances mismatch");
        }

        Ok(())
    }
}
