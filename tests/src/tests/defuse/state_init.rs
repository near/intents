use crate::tests::defuse::env::{Env, env};
use defuse_fees::Pips;
use defuse_randomness::Rng;
use defuse_sandbox::{
    extensions::{
        defuse::{
            DefuseDeployerExt, DefuseExt, DefuseSignerExt, DoAuthCallArgs,
            contract::{
                Contract as DefuseContract,
                config::{DefuseConfig, RolesConfig},
            },
            core::{fees::FeesConfig, intents::auth::AuthCall},
        },
        mt_receiver::MtReceiverStubDeployerExt,
    },
    kit::{AccountId, ExecutionStatus, Gas, GlobalContractId, NearToken, StateInit, StateInitV1},
};
use defuse_test_utils::{
    random::rng,
    wasms::{DEFUSE_WASM, MT_RECEIVER_STUB_WASM},
};
use futures::stream::{self, StreamExt};

use rstest::rstest;
use std::collections::BTreeMap;

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
        global_contract_id: &GlobalContractId,
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
            code: global_contract_id.clone(),
            data: raw_state,
        })
    }
}

fn generate_auth_intent(
    global_contract_id: &GlobalContractId,
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
    global_contract_id: &GlobalContractId,
    rng: &mut impl Rng,
    min_gas: Option<Gas>,
) -> (AccountId, AuthCall) {
    let GlobalContractId::AccountId(account_id) = global_contract_id else {
        panic!("expected AccountId-based global contract id");
    };
    let max_payload = helpers::max_single_entry_payload(account_id);
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
    #[future(awt)] env: Env,
    mut rng: impl Rng,
    #[case] gas: Option<Gas>,
) {
    let global_contract_id = env
        .deploy_mt_receiver_stub_global("g", MT_RECEIVER_STUB_WASM.clone())
        .await
        .unwrap();

    let (account, intent) =
        auth_call_with_max_possible_state_init(&global_contract_id, &mut rng, gas);

    let user = env.create_named_user("user1").await;

    // Register defuse with WNEAR and deposit WNEAR to user's defuse account
    env.initial_ft_storage_deposit(vec![user.account_id()], vec![])
        .await;
    env.defuse_ft_deposit_to(
        env.wnear.contract_id(),
        NearToken::from_near(1).as_yoctonear(),
        user.account_id(),
        None,
    )
    .await
    .unwrap();

    let signed_intent = user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    let result = env
        .defuse_execute_intents(env.defuse.contract_id(), [signed_intent])
        .await
        .unwrap();

    let on_auth_result = result
        .receipts_outcome
        .iter()
        .find(|outcome| outcome.outcome.executor_id == account)
        .unwrap();

    assert!(matches!(
        on_auth_result.outcome.status,
        ExecutionStatus::SuccessValue(_)
    ));
}

#[rstest]
#[case(None)]
#[case(Some(Gas::from_tgas(5)))]
#[case(Some(Gas::from_tgas(10)))]
#[case(Some(Gas::from_tgas(100)))]
#[tokio::test]
async fn benchmark_gas_used_by_do_auth_call_callback(
    #[future(awt)] env: Env,
    mut rng: impl Rng,
    #[case] gas: Option<Gas>,
) {
    // NOTE: when do_auth_call is scheduled as callback to withdraw (because of
    // AuthCall::storage_deposit > 0) it needs to check status of withdrawal. We can't trigger
    // it in this case so we need to subtract gas for promise read (it's around 0.1Tgas) with
    // some overhead.
    const NEAR_WITHDRAW_PROMISE_READ_OVERHEAD: Gas = Gas::from_tgas(1);

    let global_contract_id = env
        .deploy_mt_receiver_stub_global("g", MT_RECEIVER_STUB_WASM.clone())
        .await
        .unwrap();

    // Deploy second defuse instance as the receiver
    let defuse = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.contract_id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.account_id().clone(),
                },
                roles: RolesConfig::default(),
            },
            DEFUSE_WASM.clone(),
        )
        .await;

    let (account, mut intent) =
        auth_call_with_max_possible_state_init(&global_contract_id, &mut rng, gas);
    // required to opt out from promise status check
    intent.attached_deposit = NearToken::from_near(0);
    let callback_gas = DefuseContract::auth_call_callback_gas(&intent)
        .unwrap()
        .saturating_sub(NEAR_WITHDRAW_PROMISE_READ_OVERHEAD);

    defuse
        .defuse_do_auth_call(
            defuse.account_id(),
            DoAuthCallArgs {
                signer_id: &account,
                auth_call: &intent,
            },
            callback_gas,
        )
        .await
        .unwrap();
}

//NOTE: this test make sure that do_auth_call can always
//create a promise (when triggered by intent execution).
//that initializes a deterministic account
//in each case its expected to create receipt for calling
//`on_auth` on deterministic account id
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
#[cfg_attr(not(feature = "long"), ignore = "`long` feature is disabled")]
#[tokio::test]
async fn test_auth_call_state_init_via_execute_intents(
    #[future(awt)] env: Env,
    mut rng: impl Rng,
    #[case] expectation: StateInitExpectation,
) {
    let (num_keys, expect_success) = match expectation {
        ExpectStateInitSucceedsForZeroBalanceAccount(n) => (n, true),
        ExpectStateInitExceedsZeroBalanceAccountStorageLimit(n) => (n, false),
    };

    let num_keys: u8 = num_keys.try_into().unwrap();
    let concurrency_limit = 10;

    let global_contract_id = env
        .deploy_mt_receiver_stub_global("g", MT_RECEIVER_STUB_WASM.clone())
        .await
        .unwrap();

    let user = env.create_named_user("user1").await;

    // Register defuse with WNEAR and deposit WNEAR to user's defuse account
    env.initial_ft_storage_deposit(vec![user.account_id()], vec![])
        .await;
    env.defuse_ft_deposit_to(
        env.wnear.contract_id(),
        NearToken::from_near(1).as_yoctonear(),
        user.account_id(),
        None,
    )
    .await
    .unwrap();

    // Sign all intents in parallel
    let sign_futures = (0..=800)
        .step_by(10)
        .map(|value_len| {
            generate_auth_intent(&global_contract_id, num_keys, value_len, &mut rng, None)
        })
        .map(|(derived_account, intent)| {
            let user = user.clone();
            let defuse_id = &env.defuse;
            async move {
                let signed_intent = user
                    .sign_defuse_payload_default(defuse_id, [intent])
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
                let root = env.clone();
                let defuse_id = env.defuse.contract_id().clone();
                async move {
                    let result = root
                        .defuse_execute_intents(&defuse_id, [signed_intent])
                        .await
                        .unwrap();

                    // Check if receipt was created on the deterministic account
                    // Returns None if no receipt, Some(status) if receipt found
                    matches!(
                        result
                            .receipts_outcome
                            .iter()
                            .find(|outcome| outcome.outcome.executor_id == derived_account)
                            .expect(
                                "should have enough gas to pay for state init and create promise"
                            )
                            .outcome
                            .status,
                        ExecutionStatus::SuccessValue(_)
                    )
                }
            });

    let results: Vec<bool> = stream::iter(futures)
        .buffer_unordered(concurrency_limit)
        .collect()
        .await;
    let success = results.contains(&true);
    assert_eq!(success, expect_success);
}

//NOTE: this test make sure that do_auth_call can always
//create a promise that initializes a deterministic account
//in each case its expected to create receipt for calling
//`on_auth` on deterministic account id
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
#[cfg_attr(not(feature = "long"), ignore = "`long` feature is disabled")]
#[tokio::test]
async fn test_auth_call_state_init_via_do_auth_call(
    #[future(awt)] env: Env,
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
    let concurrency_limit = 10;

    let global_contract_id = env
        .deploy_mt_receiver_stub_global("g", MT_RECEIVER_STUB_WASM.clone())
        .await
        .unwrap();

    // Deploy second defuse instance as the receiver
    let defuse = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.contract_id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.account_id().clone(),
                },
                roles: RolesConfig::default(),
            },
            DEFUSE_WASM.clone(),
        )
        .await;

    // Execute all do_auth_call in parallel
    let futures = (0..=800)
        .step_by(10)
        .map(|value_len| {
            let (account_id, mut auth_intent) =
                generate_auth_intent(&global_contract_id, num_keys, value_len, &mut rng, None);
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
                    .defuse_do_auth_call(
                        defuse.account_id(),
                        DoAuthCallArgs {
                            signer_id: &account_id,
                            auth_call: &auth_intent,
                        },
                        callback_gas,
                    )
                    .await
                    .unwrap();

                // Verify receipt was created on derived account
                matches!(
                    result
                        .receipts_outcome
                        .iter()
                        .find(|outcome| outcome.outcome.executor_id == account_id)
                        .expect("should have enough gas to pay for state init and create promise")
                        .outcome
                        .status,
                    ExecutionStatus::SuccessValue(_)
                )
            }
        });

    let results: Vec<bool> = stream::iter(futures)
        .buffer_unordered(concurrency_limit)
        .collect()
        .await;
    let success = results.contains(&true);
    assert_eq!(success, expect_success);
}
