use super::DEFUSE_WASM;
use crate::{
    tests::defuse::{
        DefuseSigner, SigningStandard, accounts::AccountManagerExt, env::Env,
        intents::ExecuteIntentsExt, state::SaltManagerExt,
    },
    utils::mt::MtExt,
};
use arbitrary::{Arbitrary, Unstructured};
use chrono::{TimeDelta, Utc};
use defuse::core::{
    Deadline, ExpirableNonce, Nonce, Salt, SaltedNonce, VersionedNonce,
    crypto::PublicKey,
    intents::{DefuseIntents, Intent, account::AddPublicKey},
};
use defuse_randomness::Rng;
use defuse_test_utils::asserts::ResultAssertsExt;
use defuse_test_utils::random::{random_bytes, rng};
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

    // // let state = env
    // //     .arbitrary_state
    // //     .as_ref()
    // //     .expect("arbitrary state should be initialized");

    // // // Make some changes existing users:
    // // let user1 = state.get_random_account(&mut rng);
    // // let user2 = state.get_random_account(&mut rng);

    // Create new users
    let user3 = &env.create_user("user3").await;
    let user4 = &env.create_user("user4").await;

    let intents = [user3, user4].map(|user| {
        user.sign_defuse_message(
            SigningStandard::arbitrary(u).unwrap(),
            env.defuse.id(),
            rng.random(),
            Deadline::MAX,
            DefuseIntents {
                intents: vec![Intent::AddPublicKey(AddPublicKey {
                    public_key: PublicKey::Ed25519(rng.random()),
                })],
            },
        )
    });

    env.defuse.execute_intents(intents).await.unwrap();
}
