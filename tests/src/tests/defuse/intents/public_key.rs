use crate::tests::defuse::SigningStandard;
use crate::tests::defuse::intents::{AccountNonceIntentEvent, ExecuteIntentsExt};
use crate::utils::fixtures::{nonce, public_key, signing_standard};
use crate::{assert_eq_event_logs, tests::defuse::DefuseSigner, tests::defuse::env::Env};
use defuse::core::{
    Deadline, Nonce,
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
    intents::{
        DefuseIntents,
        account::{AddPublicKey, RemovePublicKey},
    },
};
use defuse_near_utils::NearSdkLog;
use rstest::rstest;
use std::borrow::Cow;

#[tokio::test]
#[rstest]
#[trace]
async fn execute_add_public_key_intent(
    nonce: Nonce,
    public_key: PublicKey,
    signing_standard: SigningStandard,
) {
    let env = Env::builder().no_registration(true).build().await;

    let new_public_key = public_key;

    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = env.user1.sign_defuse_message(
        signing_standard,
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![add_public_key_intent.into()],
        },
    );

    let result = env
        .defuse
        .execute_intents([add_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq_event_logs!(
        result.logs().to_vec(),
        [
            DefuseEvent::PublicKeyAdded(AccountEvent::new(
                env.user1.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key),
                },
            ))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &add_public_key_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn execute_remove_public_key_intent(
    #[from(nonce)] add_nonce: Nonce,
    #[from(nonce)] remove_nonce: Nonce,
    public_key: PublicKey,
    #[from(signing_standard)] add_signing_standard: SigningStandard,
    #[from(signing_standard)] remove_signing_standard: SigningStandard,
) {
    let env = Env::builder().no_registration(true).build().await;

    let new_public_key = public_key;
    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = env.user1.sign_defuse_message(
        add_signing_standard,
        env.defuse.id(),
        add_nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![add_public_key_intent.into()],
        },
    );

    env.defuse
        .execute_intents([add_public_key_payload])
        .await
        .unwrap();

    let remove_public_key_intent = RemovePublicKey {
        public_key: new_public_key,
    };

    let remove_public_key_payload = env.user1.sign_defuse_message(
        remove_signing_standard,
        env.defuse.id(),
        remove_nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![remove_public_key_intent.into()],
        },
    );

    let result = env
        .defuse
        .execute_intents([remove_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq_event_logs!(
        result.logs().to_vec(),
        [
            DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                env.user1.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key),
                },
            ))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), remove_nonce, &remove_public_key_payload)
                .into_event_log(),
        ]
    );
}
