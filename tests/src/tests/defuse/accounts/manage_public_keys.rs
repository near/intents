use std::borrow::Cow;

use defuse::core::{
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
};
use defuse_near_utils::NearSdkLog;
use rstest::rstest;

use crate::{
    assert_eq_event_logs,
    tests::defuse::{accounts::AccountManagerExt, env::Env},
    utils::{fixtures::public_key, test_log::TestLog},
};

#[tokio::test]
#[rstest]
#[trace]
async fn test_add_public_key(public_key: PublicKey) {
    let env = Env::builder().build().await;

    assert!(
        !env.defuse
            .has_public_key(env.user1.id(), &public_key)
            .await
            .unwrap()
    );

    let result = env
        .user1
        .call(env.defuse.id(), "add_public_key")
        .deposit(near_sdk::NearToken::from_yoctonear(1))
        .args_json(serde_json::json!({
            "public_key": public_key,
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let test_log = TestLog::from(result);

    assert_eq_event_logs!(
        test_log.logs().to_vec(),
        [DefuseEvent::PublicKeyAdded(AccountEvent::new(
            env.user1.id(),
            PublicKeyEvent {
                public_key: Cow::Borrowed(&public_key),
            },
        ))
        .to_near_sdk_log(),]
    );

    assert!(
        env.defuse
            .has_public_key(env.user1.id(), &public_key)
            .await
            .unwrap()
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn test_add_and_remove_public_key(public_key: PublicKey) {
    let env = Env::builder().build().await;

    env.user1
        .add_public_key(env.defuse.id(), public_key)
        .await
        .unwrap();

    assert!(
        env.defuse
            .has_public_key(env.user1.id(), &public_key)
            .await
            .unwrap()
    );

    let result = env
        .user1
        .call(env.defuse.id(), "remove_public_key")
        .deposit(near_sdk::NearToken::from_yoctonear(1))
        .args_json(serde_json::json!({
            "public_key": public_key,
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let test_log = TestLog::from(result);

    assert_eq_event_logs!(
        test_log.logs().to_vec(),
        [DefuseEvent::PublicKeyRemoved(AccountEvent::new(
            env.user1.id(),
            PublicKeyEvent {
                public_key: Cow::Borrowed(&public_key),
            },
        ))
        .to_near_sdk_log(),]
    );

    assert!(
        !env.defuse
            .has_public_key(env.user1.id(), &public_key)
            .await
            .unwrap()
    );
}
