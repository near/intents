use crate::{tests::defuse::{DefuseExt, DefuseSigner}, tests::defuse::accounts::AccountManagerExt, tests::defuse::env::Env};
use crate::tests::defuse::intents::ExecuteIntentsExt;
use crate::tests::defuse::SigningStandard;
use defuse_crypto::PublicKey;
use defuse::{
    core::{
        Deadline,
        intents::{
            DefuseIntents,
            account::{AddPublicKey, RemovePublicKey},
        },
    },
};
use defuse_randomness::Rng;
use defuse_test_utils::random::rng;
use rstest::rstest;

/// Test that verifies AddPublicKey intent emits the PublicKeyAdded event exactly once
/// when executed. This test ensures no duplicate events occur due to the special handling
/// in defuse/src/contract/intents/execute.rs:19-23 where PublicKeyAdded/Removed events
/// are filtered out in the on_event callback to prevent duplicates.
#[tokio::test]
#[rstest]
#[trace]
async fn execute_add_public_key_intent_no_duplicate_events(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .no_registration(true)
        .build()
        .await;

    let nonce = rng.random();

    // Step 1: Generate a new public key to add
    // We'll use a randomly generated Ed25519 public key
    let mut random_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut random_key_bytes);
    let new_public_key = PublicKey::Ed25519(random_key_bytes);

    // Step 2: Create AddPublicKey intent
    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = env.user1.sign_defuse_message(
        SigningStandard::default(),
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![add_public_key_intent.into()],
        },
    );

    // Step 3: Execute the intent
    let result = env
        .defuse
        .execute_intents([add_public_key_payload.clone()])
        .await
        .unwrap();

    // Step 4: Collect all logs from receipts (where the actual execution happens)
    let all_receipt_logs: Vec<&String> = result
        .logs_and_gas_burnt_in_receipts()
        .iter()
        .flat_map(|(logs, _gas)| logs.iter())
        .collect();

    // Step 5: Verify that PublicKeyAdded event appears exactly once in the logs
    let public_key_added_count = all_receipt_logs
        .iter()
        .filter(|log| log.contains("\"event\":\"public_key_added\""))
        .count();

    assert_eq!(
        public_key_added_count, 1,
        "PublicKeyAdded event should appear exactly once in logs, but appeared {} times.\nAll logs: {:?}",
        public_key_added_count,
        all_receipt_logs
    );

    // Step 6: Verify the event contains the correct account_id and public_key
    let public_key_added_log = all_receipt_logs
        .iter()
        .find(|log| log.contains("\"event\":\"public_key_added\""))
        .expect("PublicKeyAdded event should be present in logs");

    // Verify the log contains the correct account ID
    assert!(
        public_key_added_log.contains(&format!("\"account_id\":\"{}\"", env.user1.id())),
        "PublicKeyAdded event should contain the correct account_id"
    );

    // Verify the log contains the correct public key
    assert!(
        public_key_added_log.contains(&format!("\"public_key\":\"{}\"", new_public_key)),
        "PublicKeyAdded event should contain the correct public_key"
    );
}

/// Test that verifies RemovePublicKey intent emits the PublicKeyRemoved event exactly once
/// when executed. This test ensures no duplicate events occur due to the special handling
/// in defuse/src/contract/intents/execute.rs:19-23 where PublicKeyAdded/Removed events
/// are filtered out in the on_event callback to prevent duplicates.
#[tokio::test]
#[rstest]
#[trace]
async fn execute_remove_public_key_intent_no_duplicate_events(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .no_registration(true)
        .build()
        .await;

    // Step 1: Generate a new public key to add and then remove
    let mut random_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut random_key_bytes);
    let new_public_key = PublicKey::Ed25519(random_key_bytes);

    // Step 2: First, execute AddPublicKey to add the key to the state
    let add_nonce = rng.random();
    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = env.user1.sign_defuse_message(
        SigningStandard::default(),
        env.defuse.id(),
        add_nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![add_public_key_intent.into()],
        },
    );

    // Execute the add intent to actually add the key
    env.defuse
        .execute_intents([add_public_key_payload])
        .await
        .unwrap();

    // Step 3: Create RemovePublicKey intent to remove the key we just added
    let remove_nonce = rng.random();
    let remove_public_key_intent = RemovePublicKey {
        public_key: new_public_key,
    };

    let remove_public_key_payload = env.user1.sign_defuse_message(
        SigningStandard::default(),
        env.defuse.id(),
        remove_nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![remove_public_key_intent.into()],
        },
    );

    // Step 4: Execute the remove intent
    let result = env
        .defuse
        .execute_intents([remove_public_key_payload.clone()])
        .await
        .unwrap();

    // Step 5: Collect all logs from receipts (where the actual execution happens)
    let all_receipt_logs: Vec<&String> = result
        .logs_and_gas_burnt_in_receipts()
        .iter()
        .flat_map(|(logs, _gas)| logs.iter())
        .collect();

    // Step 6: Verify that PublicKeyRemoved event appears exactly once in the logs
    let public_key_removed_count = all_receipt_logs
        .iter()
        .filter(|log| log.contains("\"event\":\"public_key_removed\""))
        .count();

    assert_eq!(
        public_key_removed_count, 1,
        "PublicKeyRemoved event should appear exactly once in logs, but appeared {} times.\nAll logs: {:?}",
        public_key_removed_count,
        all_receipt_logs
    );

    // Step 7: Verify the event contains the correct account_id and public_key
    let public_key_removed_log = all_receipt_logs
        .iter()
        .find(|log| log.contains("\"event\":\"public_key_removed\""))
        .expect("PublicKeyRemoved event should be present in logs");

    // Verify the log contains the correct account ID
    assert!(
        public_key_removed_log.contains(&format!("\"account_id\":\"{}\"", env.user1.id())),
        "PublicKeyRemoved event should contain the correct account_id"
    );

    // Verify the log contains the correct public key
    assert!(
        public_key_removed_log.contains(&format!("\"public_key\":\"{}\"", new_public_key)),
        "PublicKeyRemoved event should contain the correct public_key"
    );
}
