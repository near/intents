use defuse_sandbox::{
    extensions::{
        defuse::{
            DefuseExt, DefuseSignerExt, HasPublicKeyArgs,
            contract::Role,
            core::{
                PublicKey as DefusePublicKey,
                amounts::Amounts,
                fees::Pips,
                intents::tokens::{FtWithdraw, Transfer},
                token_id::{TokenId, nep141::Nep141TokenId},
            },
        },
        mt::{Mt, MtBatchBalanceOfArgs},
    },
    kit::{AccountIdRef, Near},
};
use defuse_test_utils::wasms::{DEFUSE_LEGACY_WASM, DEFUSE_WASM};
use rstest::rstest;
use std::collections::BTreeMap;

use crate::tests::defuse::env::{Env, env};

async fn balance_of(
    near: &Near,
    defuse_id: impl AsRef<AccountIdRef>,
    account_id: impl AsRef<AccountIdRef>,
    token_id: &str,
) -> u128 {
    near.contract::<Mt>(defuse_id.as_ref())
        .mt_batch_balance_of(MtBatchBalanceOfArgs {
            account_id: account_id.as_ref(),
            token_ids: &[token_id.to_string()],
        })
        .await
        .unwrap()
        .into_iter()
        .next()
        .map(|v| v.0)
        .unwrap_or(0)
}

#[rstest]
#[tokio::test]
async fn test_upgrade_with_persistence(
    #[with(Env::builder().deployer_as_super_admin().defuse_wasm(DEFUSE_LEGACY_WASM.clone()))]
    #[future(awt)]
    env: Env,
) {
    let (user1, user2) = futures::join!(env.create_user(), env.create_user());
    let ft = env.create_named_token("testtoken").await;

    env.initial_ft_storage_deposit([user1.account_id(), user2.account_id()], [ft.contract_id()])
        .await;

    let deposit_amount = 10_000u128;
    futures::future::try_join_all([
        env.defuse_ft_deposit_to(ft.contract_id(), deposit_amount, user1.account_id(), None),
        env.defuse_ft_deposit_to(ft.contract_id(), deposit_amount, user2.account_id(), None),
    ])
    .await
    .unwrap();

    let token_id = TokenId::Nep141(Nep141TokenId::new(ft.contract_id().clone())).to_string();

    let get_pubkey = |near: &Near| {
        let near_pubkey = near.public_key().expect("must have signer");
        DefusePublicKey::Ed25519(
            *near_pubkey
                .as_ed25519_bytes()
                .expect("ed25519 key required"),
        )
    };

    // record state before upgrade
    assert_eq!(
        balance_of(
            &env,
            env.defuse.contract_id(),
            user1.account_id(),
            &token_id
        )
        .await,
        deposit_amount
    );
    assert_eq!(
        balance_of(
            &env,
            env.defuse.contract_id(),
            user2.account_id(),
            &token_id
        )
        .await,
        deposit_amount
    );

    assert!(
        env.defuse
            .has_public_key(HasPublicKeyArgs {
                account_id: user1.account_id(),
                public_key: &get_pubkey(&user1),
            })
            .await
            .unwrap()
    );
    assert!(
        env.defuse
            .has_public_key(HasPublicKeyArgs {
                account_id: user2.account_id(),
                public_key: &get_pubkey(&user2),
            })
            .await
            .unwrap()
    );

    let fee_before = env.defuse.fee().await.unwrap();

    env.upgrade_defuse(DEFUSE_WASM.clone()).await;

    // state persists after upgrade
    assert_eq!(
        balance_of(
            &env,
            env.defuse.contract_id(),
            user1.account_id(),
            &token_id
        )
        .await,
        deposit_amount
    );
    assert_eq!(
        balance_of(
            &env,
            env.defuse.contract_id(),
            user2.account_id(),
            &token_id
        )
        .await,
        deposit_amount
    );

    assert!(
        env.defuse
            .has_public_key(HasPublicKeyArgs {
                account_id: user1.account_id(),
                public_key: &get_pubkey(&user1),
            })
            .await
            .unwrap()
    );
    assert!(
        env.defuse
            .has_public_key(HasPublicKeyArgs {
                account_id: user2.account_id(),
                public_key: &get_pubkey(&user2),
            })
            .await
            .unwrap()
    );

    assert_eq!(env.defuse.fee().await.unwrap(), fee_before);

    // existing user can still receive deposits
    let extra = 5_000u128;
    env.defuse_ft_deposit_to(ft.contract_id(), extra, user1.account_id(), None)
        .await
        .unwrap();

    assert_eq!(
        balance_of(
            &env,
            env.defuse.contract_id(),
            user1.account_id(),
            &token_id
        )
        .await,
        deposit_amount + extra
    );

    // Transfer: user1 sends tokens to user2 within defuse
    let transfer_amount = 1_000u128;
    let transfer_payload = user1
        .sign_defuse_payload_default(
            &env.defuse,
            [Transfer {
                receiver_id: user2.account_id().clone(),
                tokens: Amounts::new(BTreeMap::from([(
                    TokenId::Nep141(Nep141TokenId::new(ft.contract_id().clone())),
                    transfer_amount,
                )])),
                memo: None,
                notification: None,
            }],
        )
        .await
        .unwrap();

    env.defuse_simulate_and_execute_intents(env.defuse.contract_id().clone(), [transfer_payload])
        .await
        .unwrap();

    assert_eq!(
        balance_of(
            &env,
            env.defuse.contract_id(),
            user1.account_id(),
            &token_id
        )
        .await,
        deposit_amount + extra - transfer_amount,
    );

    assert_eq!(
        balance_of(
            &env,
            env.defuse.contract_id(),
            user2.account_id(),
            &token_id
        )
        .await,
        deposit_amount + transfer_amount,
    );

    // FtWithdraw: user2 withdraws tokens from defuse to their FT account
    let withdraw_amount = 500u128;
    let withdraw_payload = user2
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.contract_id().clone(),
                receiver_id: user2.account_id().clone(),
                amount: withdraw_amount.into(),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    env.defuse_simulate_and_execute_intents(env.defuse.contract_id().clone(), [withdraw_payload])
        .await
        .unwrap();

    assert_eq!(
        balance_of(
            &env,
            env.defuse.contract_id(),
            user2.account_id(),
            &token_id
        )
        .await,
        deposit_amount + transfer_amount - withdraw_amount,
    );

    // new user can register and deposit
    let user3 = env.create_user().await;
    env.initial_ft_storage_deposit([user3.account_id()], [ft.contract_id()])
        .await;

    env.defuse_ft_deposit_to(ft.contract_id(), deposit_amount, user3.account_id(), None)
        .await
        .unwrap();

    assert_eq!(
        balance_of(
            &env,
            env.defuse.contract_id(),
            user3.account_id(),
            &token_id
        )
        .await,
        deposit_amount
    );

    // acl and fee management still works
    env.defuse_acl_grant_role(
        env.defuse.contract_id().clone(),
        Role::FeesManager,
        user1.account_id().clone(),
    )
    .await
    .expect("failed to grant role after upgrade");

    user1
        .defuse_set_fee(
            env.defuse.contract_id().clone(),
            Pips::from_pips(100).unwrap(),
        )
        .await
        .expect("failed to set fee after upgrade");

    assert_ne!(env.defuse.fee().await.unwrap(), fee_before);
}
