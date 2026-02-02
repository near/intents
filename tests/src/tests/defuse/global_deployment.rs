use std::collections::BTreeMap;

use defuse_randomness::Rng;
use defuse_sandbox::{MtExt, MtReceiverStubExt, sandbox};
use defuse_test_utils::random::rng;
use multi_token_receiver_stub::MTReceiverMode;
use rstest::rstest;

use crate::env::MT_RECEIVER_STUB_WASM;

#[rstest]
#[tokio::test]
async fn different_states_produce_different_addresses(
    #[future(awt)] sandbox: defuse_sandbox::Sandbox,
) -> anyhow::Result<()> {
    let root = sandbox.root();

    let global_contract = root
        .deploy_mt_receiver_stub_global("mt-receiver-global", MT_RECEIVER_STUB_WASM.clone())
        .await?;

    let mut state_a = BTreeMap::new();
    state_a.insert(b"key".to_vec(), b"value_a".to_vec());

    let mut state_b = BTreeMap::new();
    state_b.insert(b"key".to_vec(), b"value_b".to_vec());

    let account_a = root
        .deploy_mt_receiver_stub_instance(global_contract.id().clone(), state_a)
        .await?;

    let account_b = root
        .deploy_mt_receiver_stub_instance(global_contract.id().clone(), state_b)
        .await?;

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

/// Converts gas (in raw units) to Tgas as f64
#[allow(clippy::cast_precision_loss)]
fn gas_to_tgas(gas: u64) -> f64 {
    #[allow(clippy::as_conversions)]
    {
        gas as f64 / 1_000_000_000_000.0
    }
}

#[rstest]
#[tokio::test]
async fn benchmark_state_init(
    #[future(awt)] sandbox: defuse_sandbox::Sandbox,
    mut rng: impl Rng,
) -> anyhow::Result<()> {
    let root = sandbox.root();

    let global_contract = root
        .deploy_mt_receiver_stub_global("mt-receiver-global", MT_RECEIVER_STUB_WASM.clone())
        .await?;

    // Pre-generate all states with random values (rng is not thread-safe)
    let states: Vec<(usize, BTreeMap<Vec<u8>, Vec<u8>>)> = (0..=800)
        .step_by(10)
        .map(|value_size| {
            let mut value = vec![0u8; value_size];
            if value_size > 0 {
                rng.fill_bytes(&mut value);
            }
            let state: BTreeMap<Vec<u8>, Vec<u8>> = [(vec![], value)].into();
            (value_size, state)
        })
        .collect();

    // Create futures and run in parallel
    let futures = states.into_iter().map(|(value_size, state)| {
        let root = root.clone();
        let global_id = global_contract.id().clone();
        async move {
            let result = root
                .deploy_mt_receiver_stub_instance_raw(global_id, state)
                .await;
            (value_size, result)
        }
    });

    let all_results = futures::future::join_all(futures).await;

    let mut results: Vec<_> = all_results
        .into_iter()
        .filter_map(|(value_size, result)| match result {
            Ok((_, exec_result)) if exec_result.is_success() => {
                Some((value_size, exec_result.total_gas_burnt.as_gas()))
            }
            _ => {
                println!("Failed at value_size={value_size}");
                None
            }
        })
        .collect();

    // Sort by value_size since parallel execution may complete out of order
    results.sort_by_key(|(size, _)| *size);

    // Print table
    println!("\n╔═══════════════════════════════════════════════╗");
    println!("║   STATE INIT BENCHMARK (single empty key)     ║");
    println!("╠═════════════════╦═════════════════════════════╣");
    println!("║ Value Size (B)  ║ Gas (Tgas)                  ║");
    println!("╠═════════════════╬═════════════════════════════╣");
    for (size, gas) in &results {
        println!("║ {:>15} ║ {:>27.2} ║", size, gas_to_tgas(*gas));
    }
    println!("╚═════════════════╩═════════════════════════════╝");

    Ok(())
}
