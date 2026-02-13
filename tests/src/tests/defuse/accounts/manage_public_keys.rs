use std::borrow::Cow;

use defuse_sandbox::extensions::defuse::contract::core::{
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
};
use near_sdk::{AsNep297Event, NearToken, serde_json::json};
use rstest::rstest;

use crate::{
    sandbox::{assert_eq_event_logs, tx::FnCallBuilder},
    tests::defuse::env::Env,
    utils::fixtures::public_key,
};
use defuse_sandbox::extensions::defuse::account_manager::{AccountManagerExt, AccountViewExt};

#[rstest]
#[trace]
#[tokio::test]
async fn test_add_public_key(public_key: PublicKey) {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    assert!(
        !env.defuse
            .has_public_key(user.id(), &public_key)
            .await
            .unwrap()
    );

    let result = user
        .tx(env.defuse.id().clone())
        .function_call(
            FnCallBuilder::new("add_public_key")
                .with_deposit(NearToken::from_yoctonear(1))
                .json_args(json!({
                    "public_key": public_key,
                })),
        )
        .exec_transaction()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    assert_eq_event_logs!(
        result.logs().clone(),
        [DefuseEvent::PublicKeyAdded(AccountEvent::new(
            user.id(),
            PublicKeyEvent {
                public_key: Cow::Borrowed(&public_key),
            },
        ))
        .to_nep297_event()
        .to_event_log(),]
    );

    assert!(
        env.defuse
            .has_public_key(user.id(), &public_key)
            .await
            .unwrap()
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn test_add_and_remove_public_key(public_key: PublicKey) {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    user.add_public_key(env.defuse.id(), &public_key)
        .await
        .unwrap();

    assert!(
        env.defuse
            .has_public_key(user.id(), &public_key)
            .await
            .unwrap()
    );

    let result = user
        .tx(env.defuse.id().clone())
        .function_call(
            FnCallBuilder::new("remove_public_key")
                .with_deposit(NearToken::from_yoctonear(1))
                .json_args(json!({
                    "public_key": public_key,
                })),
        )
        .exec_transaction()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    assert_eq_event_logs!(
        result.logs().clone(),
        [DefuseEvent::PublicKeyRemoved(AccountEvent::new(
            user.id(),
            PublicKeyEvent {
                public_key: Cow::Borrowed(&public_key),
            },
        ))
        .to_nep297_event()
        .to_event_log(),]
    );

    assert!(
        !env.defuse
            .has_public_key(user.id(), &public_key)
            .await
            .unwrap()
    );
}
