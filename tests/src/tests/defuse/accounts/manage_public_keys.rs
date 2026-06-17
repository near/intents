use std::borrow::Cow;

use defuse_sandbox::extensions::defuse::{
    DefuseExt, HasPublicKeyArgs,
    core::{
        PublicKey,
        accounts::{AccountEvent, PublicKeyEvent},
        events::DefuseEvent,
        intents::MaybeIntentEvent,
    },
};
use defuse_test_utils::fixtures::public_key;
use near_sdk::AsNep297Event;
use rstest::rstest;

use crate::tests::defuse::env::Env;

#[rstest]
#[trace]
#[tokio::test]
async fn test_add_public_key(public_key: PublicKey) {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    assert!(
        !env.defuse
            .has_public_key(HasPublicKeyArgs {
                account_id: user.account_id(),
                public_key: &public_key,
            })
            .await
            .unwrap()
    );

    let result = user
        .defuse_add_public_key(env.defuse.contract_id(), public_key)
        .await
        .unwrap();

    let event = DefuseEvent::PublicKeyAdded(MaybeIntentEvent::new_fn_call(AccountEvent::new(
        user.account_id(),
        PublicKeyEvent {
            public_key: Cow::Borrowed(&public_key),
        },
    )))
    .to_nep297_event()
    .to_event_log();

    assert_eq!(result.logs().clone(), [event]);

    assert!(
        env.defuse
            .has_public_key(HasPublicKeyArgs {
                account_id: user.account_id(),
                public_key: &public_key,
            })
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

    user.defuse_add_public_key(env.defuse.contract_id(), public_key)
        .await
        .unwrap();

    assert!(
        env.defuse
            .has_public_key(HasPublicKeyArgs {
                account_id: user.account_id(),
                public_key: &public_key,
            })
            .await
            .unwrap()
    );

    let result = user
        .defuse_remove_public_key(env.defuse.contract_id(), public_key)
        .await
        .unwrap();

    let event = DefuseEvent::PublicKeyRemoved(MaybeIntentEvent::new_fn_call(AccountEvent::new(
        user.account_id(),
        PublicKeyEvent {
            public_key: Cow::Borrowed(&public_key),
        },
    )))
    .to_nep297_event()
    .to_event_log();

    assert_eq!(result.logs().clone(), [event]);

    assert!(
        !env.defuse
            .has_public_key(HasPublicKeyArgs {
                account_id: user.account_id(),
                public_key: &public_key,
            })
            .await
            .unwrap()
    );
}
