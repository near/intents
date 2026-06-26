use std::collections::BTreeMap;

use defuse_sandbox::{
    extensions::{
        mt::{MtExt, MtOnTransferArgs},
        mt_receiver::MtReceiverStubDeployerExt,
    },
    kit::NearToken,
    nep616::DeployDeterministicAccountExt,
};
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::json_types::U128;
use rstest::rstest;

use defuse_test_utils::wasms::MT_RECEIVER_STUB_WASM;

use crate::tests::defuse::env::{Env, env};

#[rstest]
#[tokio::test]
async fn different_states_produce_different_addresses(
    #[future(awt)] env: Env,
) -> anyhow::Result<()> {
    let global_contract_id = env
        .deploy_mt_receiver_stub_global("mt-receiver-global", MT_RECEIVER_STUB_WASM.clone())
        .await?;

    let mut state_a = BTreeMap::new();
    state_a.insert(b"key".to_vec(), b"value_a".to_vec());

    let mut state_b = BTreeMap::new();
    state_b.insert(b"key".to_vec(), b"value_b".to_vec());

    let account_a = env
        .deploy_deterministic_account(
            global_contract_id.clone(),
            state_a,
            NearToken::from_yoctonear(0),
        )
        .await?;

    let account_b = env
        .deploy_deterministic_account(global_contract_id, state_b, NearToken::from_yoctonear(0))
        .await?;

    assert_ne!(
        account_a, account_b,
        "Different states should produce different deterministic account IDs"
    );

    let accept_all_msg = serde_json::to_string(&MTReceiverMode::AcceptAll).unwrap();

    let (_, refunds_a) = env
        .mt_on_transfer(
            account_a.clone(),
            MtOnTransferArgs {
                sender_id: env.account_id(),
                previous_owner_ids: &[],
                token_ids: &["token1".to_string()],
                amounts: &[100],
                msg: &accept_all_msg,
            },
        )
        .await?;
    assert_eq!(refunds_a, vec![U128(0)], "AcceptAll should return 0 refund");

    let (_, refunds_b) = env
        .mt_on_transfer(
            account_b,
            MtOnTransferArgs {
                sender_id: env.account_id(),
                previous_owner_ids: &[],
                token_ids: &["token1".to_string()],
                amounts: &[200],
                msg: &accept_all_msg,
            },
        )
        .await?;
    assert_eq!(refunds_b, vec![U128(0)], "AcceptAll should return 0 refund");

    let refund_all_msg = serde_json::to_string(&MTReceiverMode::RefundAll).unwrap();
    let (_, refunds_refund) = env
        .mt_on_transfer(
            account_a,
            MtOnTransferArgs {
                sender_id: env.account_id(),
                previous_owner_ids: &[],
                token_ids: &["token1".to_string()],
                amounts: &[500],
                msg: &refund_all_msg,
            },
        )
        .await?;
    assert_eq!(refunds_refund, vec![U128(500)]);

    Ok(())
}
