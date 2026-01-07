use defuse::core::{
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
    intents::account::{AddPublicKey, RemovePublicKey},
};
use near_sdk::AsNep297Event;
use rstest::rstest;
use std::borrow::Cow;

use crate::intents::AccountNonceIntentEvent;
use defuse_tests::{
    contract_extensions::defuse::intents::ExecuteIntentsExt, defuse_signer::DefuseSignerExt,
    env::Env, nonce::ExtractNonceExt, utils::fixtures::public_key,
};

#[rstest]
#[trace]
#[tokio::test]
async fn execute_add_public_key_intent(public_key: PublicKey) {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let new_public_key = public_key;

    let add_public_key_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [AddPublicKey {
                public_key: new_public_key,
            }],
        )
        .await
        .unwrap();
    let nonce = add_public_key_payload.extract_nonce().unwrap();

    let result = env
        .simulate_and_execute_intents(env.defuse.id(), [add_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.logs().clone(),
        vec![
            DefuseEvent::PublicKeyAdded(AccountEvent::new(
                user.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key),
                },
            ))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user.id(), nonce, &add_public_key_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn execute_remove_public_key_intent(public_key: PublicKey) {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let new_public_key = public_key;
    let add_public_key_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [AddPublicKey {
                public_key: new_public_key,
            }],
        )
        .await
        .unwrap();
    let _add_nonce = add_public_key_payload.extract_nonce().unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [add_public_key_payload])
        .await
        .unwrap();

    let remove_public_key_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [RemovePublicKey {
                public_key: new_public_key,
            }],
        )
        .await
        .unwrap();
    let remove_nonce = remove_public_key_payload.extract_nonce().unwrap();

    let result = env
        .simulate_and_execute_intents(env.defuse.id(), [remove_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.logs().clone(),
        vec![
            DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                user.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key),
                },
            ))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user.id(), remove_nonce, &remove_public_key_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}
