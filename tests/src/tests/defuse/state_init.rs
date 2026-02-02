use std::collections::BTreeMap;

use crate::env::{Env, MT_RECEIVER_STUB_WASM};
use crate::extensions::defuse::contract::contract::consts::STATE_INIT_GAS;
use crate::extensions::defuse::contract::core::intents::auth::AuthCall;
use crate::extensions::defuse::contract::core::intents::tokens::{NotifyOnTransfer, Transfer};
use crate::extensions::defuse::contract::core::payload::multi::MultiPayload;
use crate::extensions::defuse::contract::core::token_id::{TokenId, nep141::Nep141TokenId};
use crate::extensions::defuse::intents::ExecuteIntentsExt;
use crate::extensions::defuse::signer::DefaultDefuseSignerExt;
use defuse_randomness::Rng;
use defuse_sandbox::{MtReceiverStubExt, SigningAccount, sandbox};
use defuse_test_utils::random::rng;
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::{
    AccountId, GlobalContractId, NearToken, state_init::StateInit, state_init::StateInitV1,
};
use rstest::rstest;

use crate::extensions::defuse::contract::core::amounts::Amounts;

// NOTE: this is the biggest possible state init
// that does not require storage staking
const ZERO_BALANCE_ACCOUNT_PAYLOAD_LEN: usize = 560;

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

    // Pre-generate all states with random values (rng is not thread-safe),
    // then create futures and run in parallel
    let futures = (0..=800)
        .step_by(10)
        .map(|value_size| {
            let mut value = vec![0u8; value_size];
            if value_size > 0 {
                rng.fill_bytes(&mut value);
            }
            let state: BTreeMap<Vec<u8>, Vec<u8>> = [(vec![], value)].into();
            (value_size, state)
        })
        .map(|(value_size, state)| {
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

fn create_state_init(rng: &mut impl Rng, global_contract_id: &AccountId) -> StateInit {
    let mut value = vec![0u8; ZERO_BALANCE_ACCOUNT_PAYLOAD_LEN];
    rng.fill_bytes(&mut value);
    let raw_state: BTreeMap<Vec<u8>, Vec<u8>> = [(vec![], value)].into();
    StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(global_contract_id.clone()),
        data: raw_state,
    })
}

async fn create_auth_intent_with_state_init(
    rng: &mut impl Rng,
    global_contract_id: &AccountId,
    user: &SigningAccount,
    env: &Env,
) -> (AccountId, MultiPayload) {
    let state_init = create_state_init(rng, global_contract_id);
    let derived_account = state_init.derive_account_id();

    let auth_call = AuthCall {
        contract_id: derived_account.clone(),
        state_init: Some(state_init),
        msg: String::new(),
        attached_deposit: NearToken::from_near(0),
        min_gas: None,
    };

    let payload = user
        .sign_defuse_payload_default(&env.defuse, [auth_call])
        .await
        .unwrap();

    (derived_account, payload)
}

async fn create_transfer_intent_with_state_init(
    rng: &mut impl Rng,
    global_contract_id: &AccountId,
    user: &SigningAccount,
    env: &Env,
    token_id: TokenId,
    amount: u128,
) -> (AccountId, MultiPayload) {
    let state_init = create_state_init(rng, global_contract_id);
    let derived_account = state_init.derive_account_id();

    let msg = serde_json::to_string(&MTReceiverMode::AcceptAll).unwrap();
    let transfer = Transfer {
        receiver_id: derived_account.clone(),
        tokens: Amounts::new(std::iter::once((token_id, amount)).collect()),
        memo: None,
        notification: Some(NotifyOnTransfer::new(msg).with_state_init(state_init)),
    };

    let payload = user
        .sign_defuse_payload_default(&env.defuse, [transfer])
        .await
        .unwrap();

    (derived_account, payload)
}

#[rstest]
#[tokio::test]
async fn benchmark_auth_call_with_state_init(mut rng: impl Rng) {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let global_contract = env
        .root()
        .deploy_mt_receiver_stub_global("mt-receiver-global", MT_RECEIVER_STUB_WASM.clone())
        .await
        .unwrap();

    let (account, intent) =
        create_auth_intent_with_state_init(&mut rng, global_contract.id(), &user, &env).await;

    let on_auth_call_gas = {
        let result = env
            .root()
            .execute_intents_raw(env.defuse.id(), [intent])
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
        on_auth_result.gas_burnt
    };

    assert!(on_auth_call_gas <= STATE_INIT_GAS);
}

#[rstest]
#[tokio::test]
async fn benchmark_transfer_with_notification_state_init(mut rng: impl Rng) {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();

    let global_contract = env
        .root()
        .deploy_mt_receiver_stub_global("mt-receiver-global", MT_RECEIVER_STUB_WASM.clone())
        .await
        .unwrap();

    let token_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));
    let (account, intent) = create_transfer_intent_with_state_init(
        &mut rng,
        global_contract.id(),
        &user,
        &env,
        token_id,
        1000,
    )
    .await;

    let on_transfer_gas = {
        let result = env
            .root()
            .execute_intents_raw(env.defuse.id(), [intent])
            .await
            .unwrap();
        assert!(result.is_success());
        let on_transfer_result = result
            .outcomes()
            .iter()
            .find(|outcome| outcome.executor_id == account)
            .copied()
            .unwrap();
        assert!(on_transfer_result.is_success());
        on_transfer_result.gas_burnt
    };

    assert!(on_transfer_gas <= STATE_INIT_GAS);
}
