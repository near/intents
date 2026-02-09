use std::collections::BTreeMap;

use defuse_sandbox::{MtExt, MtReceiverStubExt, sandbox};
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::serde_json;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn different_states_produce_different_addresses(
    #[future(awt)] sandbox: defuse_sandbox::Sandbox,
) -> anyhow::Result<()> {
    let root = sandbox.root();

    let global_contract_id = root
        .deploy_mt_receiver_stub_global("mt-receiver-global")
        .await;

    let mut state_a = BTreeMap::new();
    state_a.insert(b"key".to_vec(), b"value_a".to_vec());

    let mut state_b = BTreeMap::new();
    state_b.insert(b"key".to_vec(), b"value_b".to_vec());

    let account_a = root
        .deploy_mt_receiver_stub_instance(global_contract_id.clone(), state_a)
        .await;

    let account_b = root
        .deploy_mt_receiver_stub_instance(global_contract_id.clone(), state_b)
        .await;

    assert_ne!(
        account_a, account_b,
        "Different states should produce different deterministic account IDs"
    );

    let accept_all_msg = serde_json::to_string(&MTReceiverMode::AcceptAll).unwrap();
    let refunds_a = root
        .mt_on_transfer(
            root.id(),
            account_a.clone(),
            [("token1".to_string(), 100u128)],
            &accept_all_msg,
        )
        .await?;
    assert_eq!(refunds_a, vec![0u128], "AcceptAll should return 0 refund");

    let refunds_b = root
        .mt_on_transfer(
            root.id(),
            account_b.clone(),
            [("token1".to_string(), 200u128)],
            &accept_all_msg,
        )
        .await?;
    assert_eq!(refunds_b, vec![0u128], "AcceptAll should return 0 refund");

    let refund_all_msg = serde_json::to_string(&MTReceiverMode::RefundAll).unwrap();
    let refunds_refund = root
        .mt_on_transfer(
            root.id(),
            account_a.clone(),
            [("token1".to_string(), 500u128)],
            &refund_all_msg,
        )
        .await?;
    assert_eq!(refunds_refund, vec![500u128],);

    Ok(())
}
