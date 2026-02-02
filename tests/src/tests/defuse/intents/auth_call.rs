use std::collections::BTreeMap;

use crate::env::{Env, MT_RECEIVER_STUB_WASM};
use crate::extensions::defuse::contract::contract::consts::STATE_INIT_GAS;
use crate::extensions::defuse::contract::core::intents::auth::AuthCall;
use crate::extensions::defuse::intents::ExecuteIntentsExt;
use crate::extensions::defuse::signer::DefaultDefuseSignerExt;
use defuse_sandbox::MtReceiverStubExt;
use near_sdk::{GlobalContractId, NearToken, state_init::StateInit, state_init::StateInitV1};
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn benchmark_auth_call_with_state_init() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let global_contract = env
        .root()
        .deploy_mt_receiver_stub_global("mt-receiver-global", MT_RECEIVER_STUB_WASM.clone())
        .await
        .unwrap();

    let state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(global_contract.id().clone()),
        data: BTreeMap::new(),
    });

    let derived_account_id = state_init.derive_account_id();

    let auth_call_intent = AuthCall {
        contract_id: derived_account_id.clone(),
        state_init: Some(state_init),
        msg: String::new(),
        attached_deposit: NearToken::from_near(0),
        min_gas: None,
    };

    let payload = user
        .sign_defuse_payload_default(&env.defuse, [auth_call_intent])
        .await
        .unwrap();

    let result = env
        .root()
        .execute_intents_raw(env.defuse.id(), [payload])
        .await
        .unwrap();

    assert!(result.is_success());

    let state_init_gas = result
        .outcomes()
        .iter()
        .find(|outcome| outcome.executor_id == derived_account_id)
        .expect("receipt for derived account")
        .gas_burnt;

    assert!(state_init_gas < STATE_INIT_GAS);
}
