use std::collections::BTreeMap;

use defuse_sandbox::{Account, Sandbox};
use defuse_sandbox_ext::MtReceiverStubAccountExt;
use near_sdk::{AccountId, borsh::{self, BorshSerialize}};

/// Helper to serialize a struct into raw state format (BTreeMap<Vec<u8>, Vec<u8>>)
fn serialize_to_raw_state<T: BorshSerialize>(value: &T) -> BTreeMap<Vec<u8>, Vec<u8>> {
    let serialized = borsh::to_vec(value).expect("serialization should succeed");
    [(b"".to_vec(), serialized)].into()
}

#[tokio::test]
async fn different_states_produce_different_addresses() {
    let sandbox = Sandbox::new("test".parse::<AccountId>().unwrap()).await;
    let root = sandbox.root();

    // Deploy global contract
    let global_contract_id = root.deploy_mt_receiver_stub_global("mt-global").await;

    // Two different state structs - using simple tuples with different values
    let state_a: (u64, String) = (42, "state_a".to_string());
    let state_b: (u64, String) = (99, "state_b".to_string());

    let raw_state_a = serialize_to_raw_state(&state_a);
    let raw_state_b = serialize_to_raw_state(&state_b);

    // Deploy both instances
    let account_a = root
        .deploy_mt_receiver_stub_instance(global_contract_id.clone(), raw_state_a)
        .await;
    let account_b = root
        .deploy_mt_receiver_stub_instance(global_contract_id.clone(), raw_state_b)
        .await;

    // Verify addresses are different
    assert_ne!(
        account_a, account_b,
        "Different states should produce different addresses"
    );

    // Verify both accounts exist
    let acc_a = Account::new(account_a.clone(), root.network_config().clone());
    let acc_b = Account::new(account_b.clone(), root.network_config().clone());

    assert!(acc_a.view().await.is_ok(), "Account A should exist");
    assert!(acc_b.view().await.is_ok(), "Account B should exist");
}
