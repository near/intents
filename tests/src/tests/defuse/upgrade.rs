use super::DEFUSE_WASM;
use crate::{
    tests::defuse::{
        DefuseSigner, SigningStandard,
        accounts::AccountManagerExt,
        env::{Env, create_random_salted_nonce},
        intents::ExecuteIntentsExt,
        state::{FeesManagerExt, SaltManagerExt},
    },
    utils::{acl::AclExt, mt::MtExt},
};
use arbitrary::{Arbitrary, Unstructured};
use chrono::{TimeDelta, Utc};
use defuse::{
    contract::Role,
    core::{
        Deadline,
        amounts::Amounts,
        crypto::PublicKey,
        fees::Pips,
        intents::{DefuseIntents, Intent, tokens::Transfer},
        token_id::{TokenId, nep141::Nep141TokenId},
    },
};
use defuse_randomness::Rng;
use defuse_test_utils::random::{random_bytes, rng};
use itertools::Itertools;
use near_sdk::AccountId;
use rstest::rstest;

#[ignore = "only for simple upgrades"]
#[tokio::test]
#[rstest]
async fn upgrade(mut rng: impl Rng) {
    let old_contract_id: AccountId = "intents.near".parse().unwrap();
    let mainnet = near_workspaces::mainnet()
        .rpc_addr("https://nearrpc.aurora.dev")
        .await
        .unwrap();

    let sandbox = near_workspaces::sandbox().await.unwrap();
    let new_contract = sandbox
        .import_contract(&old_contract_id, &mainnet)
        .with_data()
        .transact()
        .await
        .unwrap();

    new_contract
        .as_account()
        .deploy(&DEFUSE_WASM)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    assert_eq!(
        new_contract
            .mt_balance_of(
                &"user.near".parse().unwrap(),
                &"non-existent-token".to_string(),
            )
            .await
            .unwrap(),
        0
    );

    for public_key in [
        PublicKey::Ed25519(rng.random()),
        PublicKey::Secp256k1(rng.random()),
        PublicKey::P256(rng.random()),
    ] {
        assert!(
            new_contract
                .has_public_key(&public_key.to_implicit_account_id(), &public_key)
                .await
                .unwrap()
        );

        assert!(
            !new_contract
                .has_public_key(new_contract.id(), &public_key)
                .await
                .unwrap()
        );
    }
}

#[rstest]
#[tokio::test]
async fn test_upgrade_with_persistence(mut rng: impl Rng, random_bytes: Vec<u8>) {
    // // initialize with persistent state and migration from legacy
    let u = &mut Unstructured::new(&random_bytes);
    let env = Env::builder().build_with_migration().await;

    // Make some changes existing users:
    let user1 = &env.create_user("test_user_0").await;
    let user2 = &env.create_user("test_user_1").await;

    // Create new users
    let user3 = &env.create_user("new_user1").await;
    let user4 = &env.create_user("new_user2").await;

    // Create new tokens
    let ft1 = env.create_token("new_ft1").await;

    // Check users
    {
        env.ft_storage_deposit_for_users(
            vec![user1.id(), user2.id(), user3.id(), user4.id()],
            &[&ft1],
        )
        .await;

        env.ft_deposit_to_root(&[&ft1]).await;

        for user in [user1, user2, user3, user4] {
            env.defuse_ft_deposit_to(&ft1, (10_000).try_into().unwrap(), user.id())
                .await
                .unwrap();
        }

        // Interactions between new and old users
        {
            let current_timestamp = Utc::now();
            let current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();

            let payloads = [user1, user2, user3, user4]
                .iter()
                .combinations(2)
                .map(|accounts| {
                    let sender = accounts[0];
                    let receiver = accounts[1];

                    let deadline = Deadline::new(
                        current_timestamp
                            .checked_add_signed(TimeDelta::days(1))
                            .unwrap(),
                    );
                    let expired_nonce =
                        create_random_salted_nonce(current_salt, deadline, &mut rng);

                    sender.sign_defuse_message(
                        SigningStandard::arbitrary(u).unwrap(),
                        env.defuse.id(),
                        expired_nonce,
                        deadline,
                        DefuseIntents {
                            intents: vec![Intent::Transfer(Transfer {
                                receiver_id: receiver.id().clone(),
                                tokens: Amounts::new(
                                    [(TokenId::Nep141(Nep141TokenId::new(ft1.clone())), 1000)]
                                        .into(),
                                ),
                                memo: None,
                            })],
                        },
                    )
                })
                .collect::<Vec<_>>();

            env.defuse.execute_intents(payloads).await.unwrap();
        }

        // Check auth_by_predecessor
        {
            // On old user
            user1
                .disable_auth_by_predecessor_id(env.defuse.id())
                .await
                .unwrap();

            assert!(
                !env.defuse
                    .is_auth_by_predecessor_id_enabled(user1.id())
                    .await
                    .unwrap()
            );

            // On new user
            user3
                .disable_auth_by_predecessor_id(env.defuse.id())
                .await
                .unwrap();

            assert!(
                !env.defuse
                    .is_auth_by_predecessor_id_enabled(user3.id())
                    .await
                    .unwrap()
            );
        }
    }

    // Check tokens
    {
        let tokens = user1.mt_tokens(env.defuse.id(), ..).await.unwrap();

        let old_tokens = &env.persistent_state.as_ref().unwrap().tokens;

        // Check old tokens
        assert!(old_tokens.iter().all(|t| {
            let token_id = TokenId::Nep141(t.clone()).to_string();

            tokens.iter().find(|ti| ti.token_id == token_id).is_some()
        }));

        // Check new token
        assert!(
            tokens
                .iter()
                .find(|ti| {
                    ti.token_id == TokenId::Nep141(Nep141TokenId::new(ft1.clone())).to_string()
                })
                .is_some()
        );
    }

    // Check fee
    {
        let fee = Pips::from_pips(100).unwrap();

        env.acl_grant_role(env.defuse.id(), Role::FeesManager, user1.id())
            .await
            .expect("failed to grant role");

        user1
            .set_fee(env.defuse.id(), fee)
            .await
            .expect("unable to set fee");

        let current_fee = env.defuse.fee(env.defuse.id()).await.unwrap();

        assert_eq!(current_fee, fee);
    }

    // Check salts
    {
        env.acl_grant_role(env.defuse.id(), Role::SaltManager, user1.id())
            .await
            .expect("failed to grant role");

        let new_salt = user1
            .update_current_salt(env.defuse.id())
            .await
            .expect("unable to rotate salt");

        let current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();

        assert_eq!(new_salt, current_salt);
    }
}
