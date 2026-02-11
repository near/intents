use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use defuse::{
    contract::Role,
    core::{
        accounts::{AccountEvent, PublicKeyEvent},
        crypto::PublicKey,
        events::DefuseEvent,
        intents::MaybeIntentEvent,
    },
};
use defuse_randomness::Rng;
use defuse_sandbox::{
    assert_a_contains_b,
    extensions::{acl::AclExt, defuse::account_manager::AccountViewExt},
    tx::FnCallBuilder,
};
use defuse_test_utils::{asserts::ResultAssertsExt, random::rng};
use near_sdk::{AsNep297Event, NearToken};
use rstest::rstest;
use serde_json::json;

use crate::tests::defuse::env::Env;
use defuse_test_utils::fixtures::public_key;

#[rstest]
#[trace]
#[tokio::test]
async fn test_force_add_public_keys(#[notrace] mut rng: impl Rng) {
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user1, user2) = futures::join!(env.create_user(), env.create_user());

    let public_keys = vec![&user1, &user2]
        .into_iter()
        .map(|u| {
            let pubkeys = (0..rng.random_range(0..10))
                .map(|_| public_key(&mut rng))
                .collect::<HashSet<_>>();

            (u.account().id(), pubkeys)
        })
        .collect::<HashMap<_, HashSet<PublicKey>>>();

    // only DAO or pubkey synchronizer can add public keys to accounts
    {
        user1
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("force_add_public_keys")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "public_keys": public_keys
                    })),
            )
            .exec_transaction()
            .await
            .unwrap()
            .into_result()
            .assert_err_contains("Insufficient permissions for method");
    }

    // Add public keys
    {
        env.acl_grant_role(
            env.defuse.id(),
            Role::UnrestrictedAccountManager,
            user1.id(),
        )
        .await
        .expect("failed to grant role");

        let result = user1
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("force_add_public_keys")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "public_keys": public_keys
                    })),
            )
            .exec_transaction()
            .await
            .unwrap()
            .into_result()
            .unwrap();

        for (account_id, keys) in &public_keys {
            for public_key in keys {
                assert!(
                    env.defuse
                        .has_public_key(account_id, public_key)
                        .await
                        .unwrap(),
                    "Public key {public_key:?} not found for account {account_id}",
                );

                let event = DefuseEvent::PublicKeyAdded(MaybeIntentEvent::new(AccountEvent::new(
                    *account_id,
                    PublicKeyEvent {
                        public_key: Cow::Borrowed(public_key),
                    },
                )))
                .to_nep297_event()
                .to_event_log();

                assert_a_contains_b!(
                    a: result.logs().clone(),
                    b: [event]
                );
            }
        }
    }
}

#[rstest]
#[trace]
#[tokio::test]
async fn test_force_add_and_remove_public_keys(#[notrace] mut rng: impl Rng) {
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user1, user2) = futures::join!(env.create_user(), env.create_user());

    let public_keys = vec![&user1, &user2]
        .into_iter()
        .map(|u| {
            let pubkeys = (0..rng.random_range(0..10))
                .map(|_| public_key(&mut rng))
                .collect::<HashSet<_>>();

            (u.account().id(), pubkeys)
        })
        .collect::<HashMap<_, HashSet<PublicKey>>>();

    // Add public keys
    {
        env.acl_grant_role(
            env.defuse.id(),
            Role::UnrestrictedAccountManager,
            user1.id(),
        )
        .await
        .expect("failed to grant role");

        user1
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("force_add_public_keys")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "public_keys": public_keys
                    })),
            )
            .exec_transaction()
            .await
            .unwrap()
            .into_result()
            .unwrap();
    }

    // only DAO or pubkey synchronizer can remove public keys from accounts
    {
        user2
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("force_remove_public_keys")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "public_keys": public_keys
                    })),
            )
            .exec_transaction()
            .await
            .unwrap()
            .into_result()
            .assert_err_contains("Insufficient permissions for method");
    }

    // Remove public keys
    {
        env.acl_grant_role(
            env.defuse.id(),
            Role::UnrestrictedAccountManager,
            user2.id(),
        )
        .await
        .expect("failed to grant role");

        let result = user2
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("force_remove_public_keys")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "public_keys": public_keys
                    })),
            )
            .exec_transaction()
            .await
            .unwrap()
            .into_result()
            .unwrap();

        for (account_id, keys) in &public_keys {
            for public_key in keys {
                assert!(
                    !env.defuse
                        .has_public_key(account_id, public_key)
                        .await
                        .unwrap(),
                    "Public key {public_key:?} found for account {account_id}",
                );

                let event =
                    DefuseEvent::PublicKeyRemoved(MaybeIntentEvent::new(AccountEvent::new(
                        *account_id,
                        PublicKeyEvent {
                            public_key: Cow::Borrowed(public_key),
                        },
                    )))
                    .to_nep297_event()
                    .to_event_log();

                assert_a_contains_b!(
                    a: result.logs().clone(),
                    b: [event]
                );
            }
        }
    }
}
