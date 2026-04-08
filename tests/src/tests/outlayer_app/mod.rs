use defuse_outlayer_app::{CodeLocation, Event, State as OutlayerState};
use defuse_sandbox::{
    Sandbox,
    api::types::transaction::actions::GlobalContractDeployMode,
    extensions::outlayer_app::{OutlayerAppExt, OutlayerAppViewExt},
    sandbox,
};
use defuse_test_utils::wasms::OUTLAYER_APP_WASM;
use near_sdk::{AsNep297Event, GlobalContractId, NearToken, env::sha256_array};
use near_sdk::{
    base64::{Engine as _, engine::general_purpose::STANDARD},
    borsh,
};
use rstest::{fixture, rstest};

pub struct OutlayerAppEnv {
    pub sandbox: Sandbox,
    pub global_id: GlobalContractId,
}

#[fixture]
pub async fn outlayer_app_env(#[future(awt)] sandbox: Sandbox) -> OutlayerAppEnv {
    let root = sandbox.root();
    root.deploy_global_contract(
        OUTLAYER_APP_WASM.clone(),
        GlobalContractDeployMode::CodeHash,
    )
    .await
    .unwrap();
    OutlayerAppEnv {
        sandbox,
        global_id: GlobalContractId::CodeHash(sha256_array(&*OUTLAYER_APP_WASM).into()),
    }
}

#[rstest]
#[tokio::test]
async fn test_deploy_and_upload(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    // Use a small dummy code blob as the "worker code"
    let dummy_code: Vec<u8> = (0u8..100).collect();
    let code_hash = sha256_array(&dummy_code);

    // Deploy contract instance via StateInit with alice as admin and pre-approved hash
    let state = OutlayerState::new(alice.id().clone()).pre_approve(code_hash);
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    // Verify initial state
    assert_eq!(instance.op_admin_id().await.unwrap(), *alice.id());
    assert_eq!(instance.op_code_hash().await.unwrap(), code_hash);
    assert!(instance.op_code().await.unwrap().is_none());
    assert!(instance.op_location().await.unwrap().is_none());

    // Admin (alice) uploads the code
    alice
        .op_upload_code(instance.id(), &dummy_code)
        .await
        .unwrap();

    // Verify code is now stored and location auto-set to OnChain
    assert_eq!(instance.op_code_hash().await.unwrap(), code_hash);
    let stored = instance.op_code().await.unwrap();
    assert_eq!(stored, Some(dummy_code.clone()));
    assert_eq!(
        instance.op_location().await.unwrap(),
        Some(CodeLocation::OnChain {
            account: instance.id().clone(),
            storage_prefix: OutlayerState::CODE_PREFIX.to_vec(),
        })
    );
}

#[rstest]
#[tokio::test]
async fn test_deploy_with_inline_code(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let dummy_code: Vec<u8> = (0u8..100).collect();
    let code_hash = sha256_array(&dummy_code);

    let (instance, _) = root
        .deploy_outlayer_app_with_inline_code(
            outlayer_app_env.global_id.clone(),
            &dummy_code,
        )
        .await
        .unwrap();

    assert_eq!(instance.op_admin_id().await.unwrap(), *root.id());
    assert_eq!(instance.op_code_hash().await.unwrap(), code_hash);
    assert_eq!(instance.op_code().await.unwrap(), Some(dummy_code));
    assert_eq!(
        instance.op_location().await.unwrap(),
        Some(CodeLocation::OnChain {
            account: instance.id().clone(),
            storage_prefix: OutlayerState::CODE_PREFIX.to_vec(),
        })
    );
}

#[rstest]
#[tokio::test]
async fn test_upload_rejects_wrong_hash(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    let dummy_code: Vec<u8> = (0u8..100).collect();
    let code_hash = sha256_array(&dummy_code);

    let state = OutlayerState::new(alice.id().clone()).pre_approve(code_hash);
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    // Upload wrong bytes (different from approved hash)
    let wrong_code: Vec<u8> = vec![1u8; 64];
    root.op_upload_code(instance.id(), &wrong_code)
        .await
        .expect_err("should reject code that doesn't match approved hash");
}

#[rstest]
#[tokio::test]
async fn test_upload_rejects_when_no_approved_hash(
    #[future(awt)] outlayer_app_env: OutlayerAppEnv,
) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    // Deploy with no pre-approved hash (zeros)
    let state = OutlayerState::new(alice.id().clone());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let dummy_code: Vec<u8> = (0u8..100).collect();
    root.op_upload_code(instance.id(), &dummy_code)
        .await
        .expect_err("should reject upload when code_hash is zeros");
}

#[rstest]
#[tokio::test]
async fn test_location_onchain_storage(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();

    let dummy_code: Vec<u8> = (0u8..100).collect();
    let code_hash = sha256_array(&dummy_code);

    let state = OutlayerState::new(root.id().clone()).pre_approve(code_hash);
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    root.op_upload_code(instance.id(), &dummy_code)
        .await
        .unwrap();

    // Confirm location is OnChain pointing at the instance itself
    let CodeLocation::OnChain {
        account,
        storage_prefix,
    } = instance
        .op_location()
        .await
        .unwrap()
        .expect("location should be set")
    else {
        panic!("expected OnChain location");
    };
    assert_eq!(account, *instance.id());
    assert_eq!(storage_prefix, OutlayerState::CODE_PREFIX);

    // Read raw storage directly via NEAR RPC view_state, bypassing the contract view function.
    // LazyOption stores borsh(Vec<u8>) at exactly State::CODE_PREFIX as the storage key.
    let state_result = near_api::Contract(account)
        .view_storage_with_prefix(OutlayerState::CODE_PREFIX)
        .fetch_from(instance.network_config())
        .await
        .unwrap();

    let entry = state_result
        .data
        .values
        .iter()
        .find(|item| item.key.0 == STANDARD.encode(OutlayerState::CODE_PREFIX))
        .expect("code storage entry not found at CODE_PREFIX key");

    let value_bytes = STANDARD.decode(&entry.value.0).unwrap();
    let stored: Vec<u8> = borsh::from_slice(&value_bytes).unwrap();
    assert_eq!(stored, dummy_code);
}

#[rstest]
#[tokio::test]
async fn test_non_admin_cannot_approve(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    let state = OutlayerState::new(alice.id().clone());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let new_hash = [1u8; 32];
    // root is not the admin, should fail
    root.op_approve(instance.id(), new_hash)
        .await
        .expect_err("non-admin should not be able to call op_approve");
}

#[rstest]
#[tokio::test]
async fn test_event_approve(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let state = OutlayerState::new(root.id().clone());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let new_hash = [42u8; 32];
    let result = root.op_approve(instance.id(), new_hash).await.unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::Approve {
                code_hash: new_hash
            }
            .to_nep297_event()
            .to_event_log()
        ]
    );
}

#[rstest]
#[tokio::test]
async fn test_event_upload_code(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let dummy_code: Vec<u8> = (0u8..100).collect();
    let code_hash = sha256_array(&dummy_code);
    let state = OutlayerState::new(root.id().clone()).pre_approve(code_hash);
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let result = root
        .op_upload_code(instance.id(), &dummy_code)
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::Upload {
                code_hash,
            }
            .to_nep297_event()
            .to_event_log(),
            Event::SetLocation {
                location: CodeLocation::OnChain {
                    account: instance.id().clone(),
                    storage_prefix: OutlayerState::CODE_PREFIX.to_vec(),
                },
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );
}

#[rstest]
#[tokio::test]
async fn test_event_set_admin_id(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();
    let state = OutlayerState::new(root.id().clone());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let result = root
        .op_set_admin_id(instance.id(), alice.id())
        .await
        .unwrap();

    // op_set_admin_id only emits Transfer — it no longer resets the approved hash
    assert_eq!(
        result.logs(),
        vec![
            Event::Transfer {
                old_admin_id: root.id().into(),
                new_admin_id: alice.id().into(),
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );
}

#[rstest]
#[tokio::test]
async fn test_event_set_location(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let state = OutlayerState::new(root.id().clone());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let location = CodeLocation::HttpUrl {
        url: "https://example.com/contract.wasm".to_string(),
    };
    let result = root
        .op_set_location(instance.id(), location.clone())
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::SetLocation { location }
                .to_nep297_event()
                .to_event_log()
        ]
    );
}

#[rstest]
#[tokio::test]
async fn test_event_deploy_with_inline_code(
    #[future(awt)] outlayer_app_env: OutlayerAppEnv,
) {
    let root = outlayer_app_env.sandbox.root();
    let dummy_code: Vec<u8> = (0u8..100).collect();
    let code_hash = sha256_array(&dummy_code);

    let (instance, result) = root
        .deploy_outlayer_app_with_inline_code(
            outlayer_app_env.global_id.clone(),
            &dummy_code,
        )
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::Upload {
                code_hash,
            }
            .to_nep297_event()
            .to_event_log(),
            Event::SetLocation {
                location: CodeLocation::OnChain {
                    account: instance.id().clone(),
                    storage_prefix: OutlayerState::CODE_PREFIX.to_vec(),
                },
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );
}
