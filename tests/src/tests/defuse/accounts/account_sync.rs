use std::borrow::Cow;

use defuse::{
    contract::Role,
    core::{
        accounts::{AccountEvent, PublicKeyEvent},
        crypto::PublicKey,
        events::DefuseEvent,
    },
    sandbox_ext::account_manager::AccountViewExt,
};
use defuse_randomness::Rng;
use defuse_sandbox::{assert_a_contains_b, extensions::acl::AclExt, tx::FnCallBuilder};
use defuse_test_utils::{asserts::ResultAssertsExt, random::rng};
use near_sdk::{AsNep297Event, NearToken};
use rstest::rstest;
use serde_json::json;

use crate::{tests::defuse::env::Env, utils::fixtures::public_key};

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
                .collect();

            (u.account().id(), pubkeys)
        })
        .collect::<Vec<(_, Vec<PublicKey>)>>();

    // only DAO or pubkey synchronizer can add public keys to accounts
    {
        user1
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("force_add_public_keys")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "entries": public_keys
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
        env.acl_grant_role(env.defuse.id(), Role::PubKeySynchronizer, user1.id())
            .await
            .expect("failed to grant role");

        let result = user1
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("force_add_public_keys")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "entries": public_keys
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

                assert_a_contains_b!(
                    a: result.logs().clone(),
                    b: [DefuseEvent::PublicKeyAdded(AccountEvent::new(
                        *account_id,
                        PublicKeyEvent {
                            public_key: Cow::Borrowed(public_key),
                        },
                    ))
                    .to_nep297_event()
                    .to_event_log(),]
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
                .collect();

            (u.account().id(), pubkeys)
        })
        .collect::<Vec<(_, Vec<PublicKey>)>>();

    // Add public keys
    {
        env.acl_grant_role(env.defuse.id(), Role::PubKeySynchronizer, user1.id())
            .await
            .expect("failed to grant role");

        user1
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("force_add_public_keys")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "entries": public_keys
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
                        "entries": public_keys
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
        env.acl_grant_role(env.defuse.id(), Role::PubKeySynchronizer, user2.id())
            .await
            .expect("failed to grant role");

        let result = user2
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("force_remove_public_keys")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "entries": public_keys
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
                    "Public key {public_key:?} not found for account {account_id}",
                );

                assert_a_contains_b!(
                    a: result.logs().clone(),
                    b: [DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                        *account_id,
                        PublicKeyEvent {
                            public_key: Cow::Borrowed(public_key),
                        },
                    ))
                    .to_nep297_event()
                    .to_event_log(),]
                );
            }
        }
    }
}
