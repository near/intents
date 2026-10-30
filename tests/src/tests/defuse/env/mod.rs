#![allow(dead_code)]

mod builder;
use std::collections::HashSet;

pub use self::builder::*;

use anyhow::{Context, Result, anyhow};
use arbitrary::Unstructured;
use defuse_core::crypto::ed25519::Ed25519PublicKey;
use defuse_randomness::{RngExt, make_true_rng};
use defuse_sandbox::{
    account::Account,
    extensions::{
        defuse::{
            DefuseClient, DefuseExt, HasPublicKeyArgs,
            core::PublicKey as DefusePublicKey,
            tokens::{DepositAction, DepositMessage},
        },
        poa::{PoAFactoryExt, PoaFactoryClient},
    },
    kit::{
        AccountId, AccountIdRef, Gas, NearToken, SigningNear,
        protocol::Action,
        signer::Signer,
        standards::{FungibleToken, FungibleTokenClient},
    },
    root,
};
use futures::future::{FutureExt, try_join_all};
use impl_tools::autoimpl;
use near_sdk::{account_id::arbitrary::ArbitraryNamedAccountId, json_types::U128};
use rstest::fixture;

const TOKEN_STORAGE_DEPOSIT: NearToken = NearToken::from_near(1);
const INITIAL_USER_BALANCE: NearToken = NearToken::from_near(10);

#[fixture]
pub async fn env(
    #[default(EnvBuilder::default())] builder: EnvBuilder,
    #[future(awt)] root: SigningNear,
) -> Env {
    builder.build(root).await
}

#[autoimpl(Deref using self.root)]
pub struct Env {
    pub root: SigningNear,

    pub defuse_near: SigningNear,
    pub wnear: FungibleTokenClient<SigningNear>,
    pub defuse: DefuseClient<SigningNear>,
    pub poa_factory: PoaFactoryClient<SigningNear>,
}

impl Env {
    pub fn builder() -> EnvBuilder {
        EnvBuilder::default()
    }

    pub async fn defuse_ft_deposit_to(
        &self,
        token_id: &AccountIdRef,
        amount: u128,
        to: &AccountIdRef,
        action: impl Into<Option<DepositAction>>,
    ) -> anyhow::Result<()> {
        let mut msg = DepositMessage::new(to.into());
        if let Some(action) = action.into() {
            msg = msg.with_action(action);
        }

        if self
            .contract::<FungibleToken>(AccountId::from(token_id))
            .transfer_call(
                self.defuse.contract_id(),
                amount,
                serde_json::to_string(&msg).unwrap(),
            )
            .gas(Gas::from_tgas(300))
            .await?
            .json::<U128>()
            .map(|v| v.0)?
            != amount
        {
            return Err(anyhow!("refunded"));
        }
        Ok(())
    }

    pub async fn create_named_token(
        &self,
        name: impl AsRef<str>,
    ) -> FungibleTokenClient<SigningNear> {
        self.poa_factory_deploy_token(self.poa_factory.contract_id(), name, None)
            .await
            .unwrap()
    }

    pub async fn create_token(&self) -> FungibleTokenClient<SigningNear> {
        let account_id = generate_random_account_id(self.poa_factory.contract_id())
            .expect("Failed to generate random account ID");
        let name = account_id
            .as_str()
            .trim_end_matches(&format!(".{}", self.poa_factory.contract_id()));

        self.create_named_token(name).await
    }

    pub async fn create_named_user(&self, name: &str) -> SigningNear {
        let account = self.create_subaccount(name, INITIAL_USER_BALANCE).await;

        let near_pubkey = account.signer().public_key();
        let defuse_pubkey = DefusePublicKey::Ed25519(Ed25519PublicKey(
            *near_pubkey
                .as_ed25519_bytes()
                .expect("ed25519 key required"),
        ));

        if !self
            .defuse
            .has_public_key(HasPublicKeyArgs {
                account_id: account.account_id(),
                public_key: &defuse_pubkey,
            })
            .await
            .expect("Failed to check public key")
        {
            account
                .defuse_add_public_key(self.defuse.contract_id().clone(), defuse_pubkey)
                .await
                .expect("Failed to add pubkey");
        }

        account
    }

    pub async fn create_user(&self) -> SigningNear {
        let account_id = generate_random_account_id(self.account_id())
            .expect("Failed to generate next account id");

        println!("Creating user account: {account_id}");
        let name = account_id
            .as_str()
            .trim_end_matches(&format!(".{}", self.account_id()));

        self.create_named_user(name).await
    }

    // if no tokens provided - only wnear storage deposit will be done
    pub async fn initial_ft_storage_deposit<'a>(
        &'a self,
        accounts: impl IntoIterator<Item: Into<AccountId>>,
        tokens: impl IntoIterator<Item = &'a AccountId>,
    ) {
        let all_accounts: HashSet<_> = accounts
            .into_iter()
            .map(Into::into)
            .chain([self.defuse.contract_id().clone()])
            .collect();

        let tokens: HashSet<_> = tokens.into_iter().collect();

        futures::try_join!(
            // Deposit FTs to root for transfers to users
            try_join_all(
                tokens
                    .iter()
                    .copied()
                    .map(|token| self.ft_deposit_to_root(token)),
            ),
            try_join_all(
                tokens
                    .into_iter()
                    .chain([self.wnear.contract_id()])
                    .flat_map(|token| {
                        let ft = self.contract::<FungibleToken>(token);
                        all_accounts.iter().map(move |account_id| {
                            ft.storage_deposit(account_id, TOKEN_STORAGE_DEPOSIT)
                                .into_future()
                                .map(move |r| {
                                    r.context(format!(
                                        "{token}::storage_deposit({{\"account_id\": \"{account_id}\"}})"
                                    ))
                                })
                        })
                    }),
            ),
        )
        .unwrap();
    }

    async fn ft_deposit_to_root(&self, token: &AccountIdRef) -> Result<()> {
        self.poa_factory_ft_deposit(
            self.poa_factory.contract_id().clone(),
            self.poa_factory.ft_name(token),
            self.account_id(),
            1_000_000_000,
            None,
            None,
        )
        .await
        .map(|_| ())
    }

    pub async fn upgrade_defuse(&self, wasm: impl Into<Vec<u8>>) {
        self.defuse_near
            .transaction(self.defuse_near.account_id())
            .add_action(Action::deploy_contract(wasm))
            .await
            .unwrap()
            .result()
            .unwrap();
    }

    pub async fn fund_account_with_near(
        &self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<()> {
        self.transaction(account_id)
            .add_action(Action::transfer(amount))
            .await?
            .result()?;
        Ok(())
    }
}

fn generate_random_account_id(parent_id: &AccountId) -> Result<AccountId> {
    let mut rng = make_true_rng();
    ArbitraryNamedAccountId::arbitrary_subaccount(
        &mut Unstructured::new(&rng.random::<[u8; 64]>()),
        Some(parent_id),
    )
    .map_err(|e| anyhow::anyhow!("failed to generate account ID : {e}"))
}
