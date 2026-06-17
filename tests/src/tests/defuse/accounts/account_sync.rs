use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use defuse_randomness::{Rng, RngExt};
use defuse_sandbox::extensions::defuse::{
    DefuseExt, HasPublicKeyArgs,
    contract::Role,
    core::{
        PublicKey,
        accounts::{AccountEvent, PublicKeyEvent},
        events::DefuseEvent,
        intents::MaybeIntentEvent,
    },
};
use defuse_test_utils::{asserts::ResultAssertsExt, fixtures::public_key, random::rng};
use near_sdk::{AccountId, AsNep297Event};
use rstest::rstest;

use crate::tests::defuse::env::Env;

fn generate_public_keys(
    mut rng: &mut impl Rng,
    users: impl IntoIterator<Item = impl Into<AccountId>>,
) -> HashMap<AccountId, HashSet<PublicKey>> {
    users
        .into_iter()
        .map(|u| {
            let pubkeys = (0..rng.random_range(0..10))
                .map(|_| public_key(&mut rng))
                .collect();

            (u.into(), pubkeys)
        })
        .collect()
}

#[rstest]
#[trace]
#[tokio::test]
async fn test_force_add_public_keys(#[notrace] mut rng: impl Rng) {
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user1, user2) = futures::join!(env.create_user(), env.create_user());

    let public_keys = generate_public_keys(
        &mut rng,
        [user1.account_id().clone(), user2.account_id().clone()],
    );

    // only DAO or pubkey synchronizer can add public keys to accounts
    {
        user1
            .defuse_force_add_public_keys(env.defuse.contract_id(), public_keys.clone())
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // Add public keys
    {
        env.defuse_acl_grant_role(
            env.defuse.contract_id(),
            Role::UnrestrictedAccountManager,
            user1.account_id(),
        )
        .await
        .expect("failed to grant role");

        let result = user1
            .defuse_force_add_public_keys(env.defuse.contract_id(), public_keys.clone())
            .await
            .unwrap();

        // the number of emitted events should be equal to the number of added public keys
        assert_eq!(
            result.logs().len(),
            public_keys.values().map(HashSet::len).sum::<usize>()
        );

        for (account_id, keys) in &public_keys {
            for public_key in keys {
                assert!(
                    env.defuse
                        .has_public_key(HasPublicKeyArgs {
                            account_id: account_id,
                            public_key: public_key
                        })
                        .await
                        .unwrap(),
                    "Public key {public_key:?} not found for account {account_id}",
                );

                let event =
                    DefuseEvent::PublicKeyAdded(MaybeIntentEvent::new_fn_call(AccountEvent::new(
                        account_id,
                        PublicKeyEvent {
                            public_key: Cow::Borrowed(public_key),
                        },
                    )))
                    .to_nep297_event()
                    .to_event_log();

                assert!(result.logs().contains(&event));
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

    let public_keys = generate_public_keys(
        &mut rng,
        [user1.account_id().clone(), user2.account_id().clone()],
    );

    // Add public keys
    {
        env.defuse_acl_grant_role(
            env.defuse.contract_id(),
            Role::UnrestrictedAccountManager,
            user1.account_id(),
        )
        .await
        .expect("failed to grant role");

        user1
            .defuse_force_add_public_keys(env.defuse.contract_id(), public_keys.clone())
            .await
            .unwrap();
    }

    // only DAO or pubkey synchronizer can remove public keys from accounts
    {
        user2
            .defuse_force_remove_public_keys(env.defuse.contract_id(), public_keys.clone())
            .await
            .assert_err_contains("Insufficient permissions for method");
    }

    // Remove public keys
    {
        env.defuse_acl_grant_role(
            env.defuse.contract_id(),
            Role::UnrestrictedAccountManager,
            user2.account_id(),
        )
        .await
        .expect("failed to grant role");

        let result = user2
            .defuse_force_remove_public_keys(env.defuse.contract_id(), public_keys.clone())
            .await
            .unwrap();

        // the number of emitted events should be equal to the number of removed public keys
        assert_eq!(
            result.logs().len(),
            public_keys.values().map(HashSet::len).sum::<usize>()
        );

        for (account_id, keys) in &public_keys {
            for public_key in keys {
                assert!(
                    !env.defuse
                        .has_public_key(HasPublicKeyArgs {
                            account_id: account_id,
                            public_key: public_key
                        })
                        .await
                        .unwrap(),
                    "Public key {public_key:?} found for account {account_id}",
                );

                let event = DefuseEvent::PublicKeyRemoved(MaybeIntentEvent::new_fn_call(
                    AccountEvent::new(
                        account_id,
                        PublicKeyEvent {
                            public_key: Cow::Borrowed(public_key),
                        },
                    ),
                ))
                .to_nep297_event()
                .to_event_log();

                assert!(result.logs().contains(&event));
            }
        }
    }
}
