use crate::env::{Env, MT_RECEIVER_STUB_WASM};
use crate::extensions::defuse::contract::core::intents::auth::AuthCall;
use crate::extensions::defuse::deployer::DefuseExt;
use crate::extensions::defuse::intents::ExecuteIntentsExt;
use crate::extensions::defuse::signer::DefaultDefuseSignerExt;
use defuse::contract::Contract as DefuseContract;
use defuse::contract::config::{DefuseConfig, RolesConfig};
use defuse::core::fees::FeesConfig;
use defuse_escrow_swap::Pips;
use defuse_randomness::Rng;
use defuse_sandbox::{FnCallBuilder, MtReceiverStubExt, sandbox};
use defuse_test_utils::random::rng;
use near_sdk::{
    AccountId, GlobalContractId, NearToken, state_init::StateInit, state_init::StateInitV1,
};
use near_sdk::{Gas, borsh};
use rstest::rstest;
use serde_json::json;
use std::collections::BTreeMap;

// NOTE: this is the biggest possible state init
// 770 - ZBA limit
// 100 - acount metadata
// 40  - storage entry
const ZERO_BALANCE_ACCOUNT_PAYLOAD_LEN: usize = 770 - 100 - 40;
const BORSH_VEC_LEN_PREFIX: usize = 4;

/// Converts gas (in raw units) to Tgas as f64
#[allow(clippy::cast_precision_loss)]
fn gas_to_tgas(gas: u64) -> f64 {
    #[allow(clippy::as_conversions)]
    {
        gas as f64 / 1_000_000_000_000.0
    }
}

/// Generates keys for state init benchmark: `[[], [1], [2], ...]`
fn generate_keys(n: u8) -> Vec<Vec<u8>> {
    std::iter::once(vec![])
        .chain((1..).map(|i| vec![i]))
        .take(n.into())
        .collect()
}

#[derive(Clone, Copy)]
enum StateInitExpectation {
    ExpectStateInitSucceedsForZeroBalanceAccount(u128),
    ExpectStateInitExceedsZeroBalanceAccountStorageLimit(u128),
}
use StateInitExpectation::*;

#[rstest]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(1))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(2))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(3))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(4))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(5))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(6))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(7))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(8))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(9))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(10))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(11))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(12))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(13))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(14))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(15))]
#[case(ExpectStateInitSucceedsForZeroBalanceAccount(16))]
// NOTE: edge case, with that many keys its not possible to create a valid state init as it exceeds
// 770 ZBA limit, storage usage formula:
//  - 100: account_entry
//  - 40: (storage_entry) empty key with empty value
//  - N*(40 + 1): N 1-byte keys with empty values
//  - 2: global_contract_id(`tg`) length
// the 770 ZBA storage limit is exceeded state init with (N=16) 17 entries
// 100 + 40 + N*(40 + 1) + 2 > 770
#[case(ExpectStateInitExceedsZeroBalanceAccountStorageLimit(17))]
#[tokio::test]
async fn benchmark_state_init(
    #[future(awt)] sandbox: defuse_sandbox::Sandbox,
    mut rng: impl Rng,
    #[case] expectation: StateInitExpectation,
) -> anyhow::Result<()> {
    let (num_keys, expect_results) = match expectation {
        ExpectStateInitSucceedsForZeroBalanceAccount(n) => (n, true),
        ExpectStateInitExceedsZeroBalanceAccountStorageLimit(n) => (n, false),
    };
    let num_keys: u8 = num_keys.try_into().unwrap();

    let root = sandbox.root();

    let global_contract = root
        .deploy_mt_receiver_stub_global("tg", MT_RECEIVER_STUB_WASM.clone())
        .await?;

    let keys = generate_keys(num_keys);

    // Pre-generate all states with random values (rng is not thread-safe),
    // then create futures and run in parallel
    let futures = (0..=800).step_by(10).map(|value_size| {
        let state: BTreeMap<Vec<u8>, Vec<u8>> = keys
            .iter()
            .map(|key| {
                let mut value = vec![0u8; value_size];
                if value_size > 0 {
                    rng.fill_bytes(&mut value);
                }
                (key.clone(), value)
            })
            .collect();
        let root = root.clone();
        let global_id = global_contract.id().clone();
        async move {
            match root
                .deploy_mt_receiver_stub_instance_raw(global_id, state)
                .await
            {
                Ok((_, exec_result)) if exec_result.is_success() => {
                    Some((value_size, exec_result.total_gas_burnt.as_gas()))
                }
                _ => {
                    println!("Failed at value_size={value_size}");
                    None
                }
            }
        }
    });

    let results: Vec<_> = futures::future::join_all(futures)
        .await
        .into_iter()
        .flatten()
        .collect();

    assert_eq!(results.is_empty(), !expect_results);

    if expect_results {
        let max_gas = results.iter().map(|(_, gas)| *gas).max().unwrap();
        let limit_with_margin = DefuseContract::STATE_INIT_GAS.as_gas() * 9 / 10;
        assert!(
            max_gas <= limit_with_margin,
            "Max gas {max_gas} exceeds 90% of STATE_INIT_GAS limit ({limit_with_margin})"
        );
    }

    // Print table
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!(
        "║   STATE INIT BENCHMARK ({num_keys} key{})                       ║",
        if num_keys == 1 { "" } else { "s" }
    );
    println!("╠═════════════════╦═════════════════════════════════════╣");
    println!("║ Value Size (B)  ║ Gas (Tgas)                          ║");
    println!("╠═════════════════╬═════════════════════════════════════╣");
    for (size, gas) in &results {
        println!("║ {:>15} ║ {:>35.2} ║", size, gas_to_tgas(*gas));
    }
    println!("╚═════════════════╩═════════════════════════════════════╝");

    Ok(())
}

fn create_state_init(rng: &mut impl Rng, global_contract_id: &AccountId) -> StateInit {
    let mut value =
        vec![
            0u8;
            ZERO_BALANCE_ACCOUNT_PAYLOAD_LEN - BORSH_VEC_LEN_PREFIX - global_contract_id.len()
        ];
    rng.fill_bytes(&mut value);
    let raw_state: BTreeMap<Vec<u8>, Vec<u8>> = [(vec![], borsh::to_vec(&value).unwrap())].into();
    StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(global_contract_id.clone()),
        data: raw_state,
    })
}

fn create_auth_intent_with_state_init(
    rng: &mut impl Rng,
    global_contract_id: &AccountId,
    min_gas: Option<Gas>,
) -> (AccountId, AuthCall) {
    let state_init = create_state_init(rng, global_contract_id);
    let derived_account = state_init.derive_account_id();

    let auth_call = AuthCall {
        contract_id: derived_account.clone(),
        state_init: Some(state_init),
        msg: String::new(),
        attached_deposit: NearToken::from_near(1),
        min_gas,
    };

    (derived_account, auth_call)
}

#[rstest]
#[case(None)]
#[case(Some(Gas::from_tgas(5)))]
#[case(Some(Gas::from_tgas(10)))]
#[case(Some(Gas::from_tgas(100)))]
#[tokio::test]
async fn benchmark_auth_call_with_state_init(mut rng: impl Rng, #[case] gas: Option<Gas>) {
    let env = Env::builder().build().await;

    let global_contract = env
        .root()
        .deploy_mt_receiver_stub_global("mt-receiver-global", MT_RECEIVER_STUB_WASM.clone())
        .await
        .unwrap();

    let (account, mut intent) =
        create_auth_intent_with_state_init(&mut rng, global_contract.id(), gas);
    intent.attached_deposit = NearToken::from_near(0);

    let user = env.create_named_user("user1").await;

    // Register defuse with WNEAR and deposit WNEAR to user's defuse account
    env.initial_ft_storage_deposit(vec![user.id()], vec![])
        .await;
    env.defuse_ft_deposit_to(
        env.wnear.id(),
        NearToken::from_near(1).as_yoctonear(),
        user.id(),
        None,
    )
    .await
    .unwrap();

    let signed_intent = user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    let result = env
        .root()
        .execute_intents_raw(env.defuse.id(), [signed_intent])
        .await
        .unwrap();
    assert!(result.is_success());
    let on_auth_result = result
        .outcomes()
        .iter()
        .find(|outcome| outcome.executor_id == account)
        .copied()
        .unwrap();
    assert!(on_auth_result.is_success());
}

//NOTE: do_auth_call schedules promise in state init in do_auth_call callcak. When promise in state
//init is created the cost of state init is charged at the moment of promise cration (it happens in
//do_auth_call). do_auth_call is  called in callback only if there AuthCall::storage_deposit > 0.
//We can benchmark if assumed value is correct by directly calling do_auth_call callback with same
//amount of gas as statically assigned to promise.
#[rstest]
#[case(None)]
#[case(Some(Gas::from_tgas(5)))]
#[case(Some(Gas::from_tgas(10)))]
#[case(Some(Gas::from_tgas(100)))]
#[tokio::test]
async fn benchmark_gas_used_by_do_auth_call_callback(mut rng: impl Rng, #[case] gas: Option<Gas>) {
    // NOTE: when do_auth_call is scheduled as callback to withdraw (because of
    // AuthCall::storage_deposit > 0) it needs to check status of withdrawal. We can't trigger
    // it in this case so we need to subtract gas for promise read (it's around 0.1Tgas) with
    // some overhead.
    const NEAR_WITHDRAW_PROMISE_READ_OVERHEAD: Gas = Gas::from_tgas(1);

    let env = Env::builder().build().await;

    let global_contract = env
        .root()
        .deploy_mt_receiver_stub_global("mt-receiver-global", MT_RECEIVER_STUB_WASM.clone())
        .await
        .unwrap();

    // Deploy second defuse instance as the receiver
    let defuse = env
        .root()
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.id().clone(),
                },
                roles: RolesConfig::default(),
            },
            crate::env::DEFUSE_WASM.clone(),
        )
        .await
        .unwrap();

    let (account, mut intent) =
        create_auth_intent_with_state_init(&mut rng, global_contract.id(), gas);
    // required to opt out from promise status check
    intent.attached_deposit = NearToken::from_near(0);
    let callback_gas = DefuseContract::auth_call_callback_gas(&intent)
        .unwrap()
        .saturating_sub(NEAR_WITHDRAW_PROMISE_READ_OVERHEAD);

    let result = defuse
        .tx(defuse.id())
        .function_call(
            FnCallBuilder::new("do_auth_call")
                .with_gas(callback_gas)
                .json_args(json!({
                    "signer_id": account,
                    "auth_call": intent
                })),
        )
        .exec_transaction()
        .await
        .unwrap();

    assert!(result.is_success());
}
