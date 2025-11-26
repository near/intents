use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::intents::{AccountNonceIntentEvent, ExecuteIntentsExt};
use crate::utils::fixtures::public_key;
use crate::utils::payload::ExtractNonceExt;
use crate::{assert_eq_event_logs, tests::defuse::env::Env};
use defuse::core::{
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
    intents::account::{AddPublicKey, RemovePublicKey},
};
use defuse_near_utils::NearSdkLog;
use rstest::rstest;
use std::borrow::Cow;

#[tokio::test]
#[rstest]
#[trace]
async fn execute_add_public_key_intent(public_key: PublicKey) {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let new_public_key = public_key;

    let add_public_key_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [AddPublicKey {
                public_key: new_public_key,
            }],
        )
        .await
        .unwrap();
    let nonce = add_public_key_payload.extract_nonce().unwrap();

    let result = env
        .defuse
        .execute_intents(env.defuse.id(), [add_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq_event_logs!(
        result.logs().to_vec(),
        [
            DefuseEvent::PublicKeyAdded(AccountEvent::new(
                user.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key),
                },
            ))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&user.id(), nonce, &add_public_key_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn execute_remove_public_key_intent(public_key: PublicKey) {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let new_public_key = public_key;
    let add_public_key_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [AddPublicKey {
                public_key: new_public_key,
            }],
        )
        .await
        .unwrap();
    let _add_nonce = add_public_key_payload.extract_nonce().unwrap();

    env.defuse
        .execute_intents(env.defuse.id(), [add_public_key_payload])
        .await
        .unwrap();

    let remove_public_key_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [RemovePublicKey {
                public_key: new_public_key,
            }],
        )
        .await
        .unwrap();
    let remove_nonce = remove_public_key_payload.extract_nonce().unwrap();

    let result = env
        .defuse
        .execute_intents(env.defuse.id(), [remove_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq_event_logs!(
        result.logs().to_vec(),
        [
            DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                user.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key),
                },
            ))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&user.id(), remove_nonce, &remove_public_key_payload)
                .into_event_log(),
        ]
    );
}
