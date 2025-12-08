use anyhow::Result;
use near_sdk::AccountId;
use near_workspaces::Account;
use std::{collections::HashSet, convert::Infallible, sync::atomic::Ordering};

use defuse::{
    contract::Role,
    core::{
        Deadline, Nonce,
        crypto::PublicKey,
        intents::{DefuseIntents, Intent, account::AddPublicKey},
        token_id::{TokenId, nep141::Nep141TokenId},
    },
    nep245::Token,
};
use defuse_randomness::{Rng, make_true_rng};
use futures::{StreamExt, TryStreamExt, future::try_join_all};

use crate::{
    tests::{
        defuse::{
            DefuseExt, DefuseSigner, SigningStandard,
            accounts::AccountManagerExt,
            env::{
                Env,
                state::{AccountWithTokens, PersistentState},
            },
            intents::ExecuteIntentsExt,
        },
        poa::factory::PoAFactoryExt,
    },
    utils::{ParentAccount, acl::AclExt, mt::MtExt},
};

impl Env {
    pub async fn upgrade_legacy(&self, reuse_accounts: bool) {
        let state = self
            .generate_storage_data()
            .await
            .expect("Failed to generate state");

        self.acl_grant_role(
            self.defuse.id(),
            Role::Upgrader,
            self.sandbox.root_account().id(),
        )
        .await
        .expect("Failed to grant upgrader role");

        self.upgrade_defuse(self.defuse.id())
            .await
            .expect("Failed to upgrade defuse");

        self.verify_storage_consistency(&state).await;

        if !reuse_accounts {
            self.next_user_index
                .store(state.accounts.len(), Ordering::Relaxed);
        }
    }

    async fn generate_storage_data(&self) -> Result<PersistentState> {
        let state = PersistentState::generate(
            self.sandbox.root_account(),
            self.poa_factory.as_account(),
            self.seed,
        );

        self.apply_tokens(&state).await?;
        self.apply_accounts(&state).await?;

        Ok(state)
    }

    async fn verify_storage_consistency(&self, state: &PersistentState) {
        futures::join!(
            self.verify_accounts_consistency(state),
            self.verify_mt_tokens_consistency(state)
        );
    }

    async fn apply_tokens(&self, state: &PersistentState) -> Result<()> {
        let tokens = state.get_tokens();
        try_join_all(tokens.iter().map(|token| self.apply_token(token)))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to apply tokens: {}", e))?;

        Ok(())
    }

    async fn apply_accounts(&self, state: &PersistentState) -> Result<()> {
        try_join_all(state.accounts.iter().map(|data| self.apply_account(data)))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to apply accounts: {}", e))?;

        Ok(())
    }

    async fn apply_token(&self, token_id: &Nep141TokenId) -> Result<()> {
        let root = self.sandbox.root_account();
        let token_name = self.poa_factory.subaccount_name(&token_id.contract_id);

        let token = root
            .poa_factory_deploy_token(self.poa_factory.id(), &token_name, None)
            .await?;

        self.ft_storage_deposit_for_accounts(&token, vec![root.id(), self.defuse.id()])
            .await?;

        self.ft_deposit_to_root(&token).await?;

        Ok(())
    }

    async fn apply_public_keys(&self, acc: &Account, data: &AccountWithTokens) -> Result<()> {
        let intents = data
            .data
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

    async fn apply_nonces(&self, acc: &Account, data: &AccountWithTokens) -> Result<()> {
        let payload = data
            .data
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

    async fn apply_token_balance(&self, acc: &Account, data: &AccountWithTokens) -> Result<()> {
        try_join_all(data.tokens.iter().map(|(token_id, balance)| async {
            self.defuse_ft_deposit_to(&token_id.contract_id, *balance, acc.id(), None)
                .await
        }))
        .await?;

        Ok(())
    }

    async fn apply_account(&self, data: (&AccountId, &AccountWithTokens)) -> Result<Account> {
        let (account_id, account) = data;
        let acc = self
            .create_named_user(&self.sandbox.subaccount_name(account_id))
            .await;

        futures::try_join!(
            self.apply_public_keys(&acc, account),
            self.apply_nonces(&acc, account),
            self.apply_token_balance(&acc, account),
        )?;

        Ok(acc)
    }

    async fn verify_accounts_consistency(&self, state: &PersistentState) {
        for (account_id, data) in &state.accounts {
            futures::join!(
                self.verify_public_keys(account_id, &data.data.public_keys),
                self.verify_nonces(account_id, &data.data.nonces),
                self.verify_account_nep141_balance(account_id, data.tokens.clone()),
            );
        }
    }

    async fn verify_public_keys(&self, account_id: &AccountId, public_keys: &HashSet<PublicKey>) {
        assert!(
            futures::stream::iter(public_keys)
                .map(Ok::<_, Infallible>)
                .try_all(|n| async { self.defuse.has_public_key(account_id, n).await.unwrap() })
                .await
                .unwrap()
        );
    }

    async fn verify_nonces(&self, account_id: &AccountId, nonces: &HashSet<Nonce>) {
        assert!(
            futures::stream::iter(nonces)
                .map(Ok::<_, Infallible>)
                .try_all(|n| async { self.defuse.is_nonce_used(account_id, n).await.unwrap() })
                .await
                .unwrap()
        );
    }

    async fn verify_account_nep141_balance(
        &self,
        account_id: &AccountId,
        tokens: impl IntoIterator<Item = (Nep141TokenId, u128)>,
    ) {
        let (tokens, amounts): (Vec<Nep141TokenId>, Vec<u128>) = tokens.into_iter().unzip();
        let tokens = tokens
            .iter()
            .map(|t| TokenId::Nep141(t.clone()).to_string())
            .collect::<Vec<_>>();

        let result = self
            .defuse
            .mt_batch_balance_of(account_id, &tokens)
            .await
            .expect("Failed to fetch balance");

        assert_eq!(result, amounts);
    }

    async fn verify_mt_tokens_consistency(&self, state: &PersistentState) {
        let expected = state
            .get_tokens()
            .into_iter()
            .map(|token_id| Token {
                token_id: TokenId::Nep141(token_id).to_string(),
                owner_id: None,
            })
            .collect::<Vec<_>>();

        let mut tokens = self
            .mt_tokens(self.defuse.id(), ..)
            .await
            .expect("Failed to fetch tokens");

        tokens.sort();

        assert_eq!(tokens, expected);
    }
}
