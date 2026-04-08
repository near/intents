use defuse_outlayer_project::{State as OutlayerState, WasmLocation};
use near_sdk::{base64::{Engine as _, engine::general_purpose::STANDARD}, borsh};
use defuse_sandbox::{
    Sandbox,
    api::types::transaction::actions::GlobalContractDeployMode,
    extensions::outlayer_project::{OutlayerProjectExt, OutlayerProjectViewExt},
    sandbox,
};
use defuse_test_utils::wasms::OUTLAYER_PROJECT_WASM;
use near_sdk::{GlobalContractId, NearToken, env::sha256_array};
use rstest::{fixture, rstest};

pub struct OutlayerProjectEnv {
    pub sandbox: Sandbox,
    pub global_id: GlobalContractId,
}

#[fixture]
pub async fn outlayer_project_env(#[future(awt)] sandbox: Sandbox) -> OutlayerProjectEnv {
    let root = sandbox.root();
    root.deploy_global_contract(
        OUTLAYER_PROJECT_WASM.clone(),
        GlobalContractDeployMode::CodeHash,
    )
    .await
    .unwrap();
    OutlayerProjectEnv {
        sandbox,
        global_id: GlobalContractId::CodeHash(sha256_array(&*OUTLAYER_PROJECT_WASM).into()),
    }
}

#[rstest]
#[tokio::test]
async fn test_deploy_and_upload(#[future(awt)] outlayer_project_env: OutlayerProjectEnv) {
    let root = outlayer_project_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    // Use a small dummy WASM blob as the "worker code"
    let dummy_wasm: Vec<u8> = (0u8..100).collect();
    let wasm_hash = sha256_array(&dummy_wasm);

    // Deploy contract instance via StateInit with alice as updater and pre-approved hash
    let state = OutlayerState::new(alice.id().clone()).pre_approve(wasm_hash);
    let instance = root
        .deploy_outlayer_project(outlayer_project_env.global_id.clone(), state)
        .await
        .unwrap();

    // Verify initial state
    assert_eq!(instance.oc_updater_id().await.unwrap(), *alice.id());
    assert_eq!(instance.oc_wasm_hash().await.unwrap(), wasm_hash);
    assert!(instance.oc_wasm().await.unwrap().is_none());
    assert!(instance.oc_location().await.unwrap().is_none());

    // Updater (alice) uploads the code
    alice.oc_upload_wasm(instance.id(), &dummy_wasm)
        .await
        .unwrap();

    // Verify code is now stored and location auto-set to OnChain
    assert_eq!(instance.oc_wasm_hash().await.unwrap(), wasm_hash);
    let stored = instance.oc_wasm().await.unwrap();
    assert_eq!(stored, Some(dummy_wasm.clone()));
    assert_eq!(
        instance.oc_location().await.unwrap(),
        Some(WasmLocation::OnChain {
            account: instance.id().clone(),
            storage_prefix: OutlayerState::WASM_PREFIX.to_vec(),
        })
    );
}

#[rstest]
#[tokio::test]
async fn test_upload_rejects_wrong_hash(#[future(awt)] outlayer_project_env: OutlayerProjectEnv) {
    let root = outlayer_project_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    let dummy_wasm: Vec<u8> = (0u8..100).collect();
    let wasm_hash = sha256_array(&dummy_wasm);

    let state = OutlayerState::new(alice.id().clone()).pre_approve(wasm_hash);
    let instance = root
        .deploy_outlayer_project(outlayer_project_env.global_id.clone(), state)
        .await
        .unwrap();

    // Upload wrong bytes (different from approved hash)
    let wrong_wasm: Vec<u8> = vec![1u8; 64];
    root.oc_upload_wasm(instance.id(), &wrong_wasm)
        .await
        .expect_err("should reject code that doesn't match approved hash");
}

#[rstest]
#[tokio::test]
async fn test_upload_rejects_when_no_approved_hash(
    #[future(awt)] outlayer_project_env: OutlayerProjectEnv,
) {
    let root = outlayer_project_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    // Deploy with no pre-approved hash (zeros)
    let state = OutlayerState::new(alice.id().clone());
    let instance = root
        .deploy_outlayer_project(outlayer_project_env.global_id.clone(), state)
        .await
        .unwrap();

    let dummy_wasm: Vec<u8> = (0u8..100).collect();
    root.oc_upload_wasm(instance.id(), &dummy_wasm)
        .await
        .expect_err("should reject upload when wasm_hash is zeros");
}

#[rstest]
#[tokio::test]
async fn test_location_onchain_storage(#[future(awt)] outlayer_project_env: OutlayerProjectEnv) {
    let root = outlayer_project_env.sandbox.root();

    let dummy_wasm: Vec<u8> = (0u8..100).collect();
    let wasm_hash = sha256_array(&dummy_wasm);

    let state = OutlayerState::new(root.id().clone()).pre_approve(wasm_hash);
    let instance = root
        .deploy_outlayer_project(outlayer_project_env.global_id.clone(), state)
        .await
        .unwrap();

    root.oc_upload_wasm(instance.id(), &dummy_wasm)
        .await
        .unwrap();

    // Confirm location is OnChain pointing at the instance itself
    let WasmLocation::OnChain { account, storage_prefix } =
        instance.oc_location().await.unwrap().expect("location should be set")
    else {
        panic!("expected OnChain location");
    };
    assert_eq!(account, *instance.id());
    assert_eq!(storage_prefix, OutlayerState::WASM_PREFIX);

    // Read raw storage directly via NEAR RPC view_state, bypassing the contract view function.
    // LazyOption stores borsh(Vec<u8>) at exactly State::WASM_PREFIX as the storage key.
    let state_result = near_api::Contract(account)
        .view_storage_with_prefix(OutlayerState::WASM_PREFIX)
        .fetch_from(instance.network_config())
        .await
        .unwrap();

    let entry = state_result
        .data
        .values
        .iter()
        .find(|item| item.key.0 == STANDARD.encode(OutlayerState::WASM_PREFIX))
        .expect("wasm storage entry not found at WASM_PREFIX key");

    let value_bytes = STANDARD.decode(&entry.value.0).unwrap();
    let stored: Vec<u8> = borsh::from_slice(&value_bytes).unwrap();
    assert_eq!(stored, dummy_wasm);
}

#[rstest]
#[tokio::test]
async fn test_non_updater_cannot_approve(#[future(awt)] outlayer_project_env: OutlayerProjectEnv) {
    let root = outlayer_project_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    let state = OutlayerState::new(alice.id().clone());
    let instance = root
        .deploy_outlayer_project(outlayer_project_env.global_id.clone(), state)
        .await
        .unwrap();

    let new_hash = [1u8; 32];
    // root is not the updater, should fail
    root.oc_approve(instance.id(), new_hash)
        .await
        .expect_err("non-updater should not be able to call oc_approve");
}
