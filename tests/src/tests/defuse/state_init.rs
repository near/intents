use crate::tests::defuse::env::Env;
use crate::wasms::{DEFUSE_WASM, MT_RECEIVER_STUB_WASM};
use defuse::contract::Contract as DefuseContract;
use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::FeesConfig,
};
use defuse_escrow_swap::Pips;
use defuse_randomness::Rng;
use defuse_sandbox::FnCallBuilder;
use defuse_sandbox::api::types::transaction::actions::GlobalContractDeployMode;
use defuse_sandbox::extensions::defuse::contract::core::intents::auth::AuthCall;
use defuse_sandbox::extensions::defuse::intents::ExecuteIntentsExt;
use defuse_sandbox::extensions::defuse::signer::DefaultDefuseSignerExt;
use defuse_test_utils::random::rng;
use futures::stream::{self, StreamExt};
use near_sdk::Gas;
use near_sdk::{
    AccountId, GlobalContractId, NearToken, serde_json::json, state_init::StateInit,
    state_init::StateInitV1,
};
use rstest::rstest;
use std::collections::BTreeMap;

use defuse_sandbox::extensions::defuse::deployer::DefuseExt;

mod helpers {
    use super::*;
    // // NOTE: this is the biggest possible state init
    // // 770 - ZBA limit
    // // 100 - acount metadata
    // // 40  - storage entry
    const ZERO_BALANCE_ACCOUNT_PAYLOAD_LEN: usize = 770 - 100 - 40;

    /// Calculates the maximum allowed payload size for a single-entry state init
    /// that fits within ZBA storage limits for the given global contract id.
    pub fn max_single_entry_payload(global_contract_id: &AccountId) -> usize {
        ZERO_BALANCE_ACCOUNT_PAYLOAD_LEN - global_contract_id.len()
    }

    /// Generates raw state data for state init benchmark
    pub fn generate_raw_state(
        keys: &[Vec<u8>],
        value_size: usize,
        rng: &mut impl Rng,
    ) -> BTreeMap<Vec<u8>, Vec<u8>> {
        keys.iter()
            .map(|key| {
                let mut value = vec![0u8; value_size];
                if value_size > 0 {
                    rng.fill_bytes(&mut value);
                    (key.clone(), value)
                } else {
                    (key.clone(), vec![])
                }
            })
            .collect()
    }

    /// Simple helper to generate a `StateInit` with given parameters
    pub fn generate_state_init(
        global_contract_id: &AccountId,
        keys_count: u8,
        value_len: usize,
        rng: &mut impl Rng,
    ) -> StateInit {
        let keys: Vec<Vec<_>> = std::iter::once(vec![])
            .chain((1..).map(|i| vec![i]))
            .take(keys_count.into())
            .collect();

        let raw_state = generate_raw_state(&keys, value_len, rng);
        StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state,
        })
    }
}

fn generate_auth_intent(
    global_contract_id: &AccountId,
    keys_count: u8,
    value_len: usize,
    rng: &mut impl Rng,
    min_gas: Option<Gas>,
) -> (AccountId, AuthCall) {
    let state_init = helpers::generate_state_init(global_contract_id, keys_count, value_len, rng);
    let derived_account = state_init.derive_account_id();
    let auth_call = AuthCall {
        contract_id: derived_account.clone(),
        state_init: Some(state_init),
        msg: String::new(),
        attached_deposit: NearToken::from_yoctonear(1),
        min_gas,
    };
    (derived_account, auth_call)
}

/// Creates an auth call intent with the maximum possible state init
/// that fits within zero balance account storage limits.
fn auth_call_with_max_possible_state_init(
    global_contract_id: &AccountId,
    rng: &mut impl Rng,
    min_gas: Option<Gas>,
) -> (AccountId, AuthCall) {
    let max_payload = helpers::max_single_entry_payload(global_contract_id);
    generate_auth_intent(global_contract_id, 1, max_payload, rng, min_gas)
}

#[derive(Clone, Copy)]
enum StateInitExpectation {
    ExpectStateInitSucceedsForZeroBalanceAccount(u128),
    ExpectStateInitExceedsZeroBalanceAccountStorageLimit(u128),
}
use StateInitExpectation::*;

#[rstest]
#[case(None)]
#[case(Some(Gas::from_tgas(5)))]
#[case(Some(Gas::from_tgas(10)))]
#[case(Some(Gas::from_tgas(100)))]
#[tokio::test]
async fn benchmark_auth_call_with_largest_possible_state_init(
    mut rng: impl Rng,
    #[case] gas: Option<Gas>,
) {
    let env = Env::builder().build().await;
    env.root()
        .deploy_global_contract(
            MT_RECEIVER_STUB_WASM.clone(),
            GlobalContractDeployMode::AccountId,
        )
        .await
        .unwrap();
    let global_contract_id = env.root().id();

    let (account, intent) =
        auth_call_with_max_possible_state_init(global_contract_id, &mut rng, gas);

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
    env.root()
        .deploy_global_contract(
            MT_RECEIVER_STUB_WASM.clone(),
            GlobalContractDeployMode::AccountId,
        )
        .await
        .unwrap();
    let global_contract_id = env.root().id();

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
            DEFUSE_WASM.clone(),
        )
        .await
        .unwrap();

    let (account, mut intent) =
        auth_call_with_max_possible_state_init(global_contract_id, &mut rng, gas);
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

//NOTE: this test make sure that do_auth_call can always
//create a promise (when triggered by intent execution).
//that initializes a deterministic account
//in each case its expected to create receipt for calling
//`on_auth` on deterministic account id
#[cfg(feature = "long")]
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
#[case(ExpectStateInitExceedsZeroBalanceAccountStorageLimit(17))]
#[tokio::test]
async fn test_auth_call_state_init_via_execute_intents(
    mut rng: impl Rng,
    #[case] expectation: StateInitExpectation,
) {
    let (num_keys, expect_success) = match expectation {
        ExpectStateInitSucceedsForZeroBalanceAccount(n) => (n, true),
        ExpectStateInitExceedsZeroBalanceAccountStorageLimit(n) => (n, false),
    };

    let num_keys: u8 = num_keys.try_into().unwrap();

    let env = Env::builder().build().await;
    env.root()
        .deploy_global_contract(
            MT_RECEIVER_STUB_WASM.clone(),
            GlobalContractDeployMode::AccountId,
        )
        .await
        .unwrap();
    let global_contract = env.root();

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

    // Sign all intents in parallel
    let sign_futures = (0..=800)
        .step_by(10)
        .map(|value_len| generate_auth_intent(global_contract, num_keys, value_len, &mut rng, None))
        .map(|(derived_account, intent)| {
            let user = user.clone();
            let defuse = env.defuse.clone();
            async move {
                let signed_intent = user
                    .sign_defuse_payload_default(&defuse, [intent])
                    .await
                    .unwrap();
                (derived_account, signed_intent)
            }
        });
    let signed_intents_with_accounts: Vec<_> = futures::future::join_all(sign_futures).await;

    // Execute all intents in parallel and check if receipt was created on derived account
    let futures =
        signed_intents_with_accounts
            .into_iter()
            .map(|(derived_account, signed_intent)| {
                let root = env.root().clone();
                let defuse_id = env.defuse.id().clone();
                async move {
                    let result = root
                        .execute_intents_raw(&defuse_id, [signed_intent])
                        .await
                        .unwrap();

                    // Check if receipt was created on the deterministic account
                    // Returns None if no receipt, Some(status) if receipt found
                    result
                        .outcomes()
                        .iter()
                        .find(|outcome| outcome.executor_id == derived_account)
                        .expect("should have enough gas to pay for state init and create promise")
                        .is_success()
                }
            });

    let results: Vec<bool> = stream::iter(futures).buffer_unordered(10).collect().await;
    let success = results.contains(&true);
    assert_eq!(success, expect_success);
}

//NOTE: this test make sure that do_auth_call can always
//create a promise that initializes a deterministic account
//in each case its expected to create receipt for calling
//`on_auth` on deterministic account id
#[cfg(feature = "long")]
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
#[case(ExpectStateInitExceedsZeroBalanceAccountStorageLimit(17))]
#[tokio::test]
async fn test_auth_call_state_init_via_do_auth_call(
    mut rng: impl Rng,
    #[case] expectation: StateInitExpectation,
) {
    // NOTE: when do_auth_call is scheduled as callback to withdraw (because of
    // AuthCall::storage_deposit > 0) it needs to check status of withdrawal. We can't trigger
    // it in this case so we need to subtract gas for promise read (it's around 0.1Tgas) with
    // some overhead.
    const NEAR_WITHDRAW_PROMISE_READ_OVERHEAD: Gas = Gas::from_tgas(1);

    let (num_keys, expect_success) = match expectation {
        ExpectStateInitSucceedsForZeroBalanceAccount(n) => (n, true),
        ExpectStateInitExceedsZeroBalanceAccountStorageLimit(n) => (n, false),
    };

    let num_keys: u8 = num_keys.try_into().unwrap();

    let env = Env::builder().build().await;
    env.root()
        .deploy_global_contract(
            MT_RECEIVER_STUB_WASM.clone(),
            GlobalContractDeployMode::AccountId,
        )
        .await
        .unwrap();
    let global_contract_id = env.root().id();

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
            DEFUSE_WASM.clone(),
        )
        .await
        .unwrap();

    // Execute all do_auth_call in parallel
    let futures = (0..=800)
        .step_by(10)
        .map(|value_len| {
            let (account_id, mut auth_intent) =
                generate_auth_intent(global_contract_id, num_keys, value_len, &mut rng, None);
            auth_intent.attached_deposit = NearToken::from_near(0);

            let callback_gas = DefuseContract::auth_call_callback_gas(&auth_intent)
                .unwrap()
                .saturating_sub(NEAR_WITHDRAW_PROMISE_READ_OVERHEAD);
            (account_id, auth_intent, callback_gas)
        })
        .map(|(account_id, auth_intent, callback_gas)| {
            let defuse = defuse.clone();
            async move {
                let result = defuse
                    .tx(defuse.id())
                    .function_call(
                        FnCallBuilder::new("do_auth_call")
                            .with_gas(callback_gas)
                            .json_args(json!({
                                "signer_id": account_id,
                                "auth_call": auth_intent
                            })),
                    )
                    .exec_transaction()
                    .await
                    .unwrap();

                // Verify receipt was created on derived account
                result
                    .outcomes()
                    .iter()
                    .find(|outcome| outcome.executor_id == account_id)
                    .expect("should have enough gas to pay for state init and create promise")
                    .is_success()
            }
        });

    let results: Vec<bool> = stream::iter(futures).buffer_unordered(10).collect().await;
    let success = results.contains(&true);
    assert_eq!(success, expect_success);
}
