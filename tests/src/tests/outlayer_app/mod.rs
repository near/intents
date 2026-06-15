use defuse_sandbox::{
    account::Account,
    extensions::outlayer_app::{
        OutlayerAppDeployerExt, OutlayerAppExt,
        contract::{Event, State as OutlayerState},
    },
    kit::{GlobalContractIdentifier, Near, PublishMode},
    root,
};
use defuse_test_utils::wasms::OUTLAYER_APP_WASM;
use near_sdk::{AsNep297Event, NearToken, env::sha256_array};
use rstest::{fixture, rstest};

const EXAMPLE_URL: &str = "https://example.com/contract.wasm";

pub struct OutlayerAppEnv {
    pub root: Near,
    pub global_id: GlobalContractIdentifier,
}

#[fixture]
pub async fn outlayer_app_env(#[future(awt)] root: Near) -> OutlayerAppEnv {
    root.publish(OUTLAYER_APP_WASM.clone(), PublishMode::Immutable)
        .await
        .unwrap()
        .result()
        .unwrap();

    OutlayerAppEnv {
        root,
        global_id: GlobalContractIdentifier::CodeHash(sha256_array(&*OUTLAYER_APP_WASM).into()),
    }
}

#[rstest]
#[tokio::test]
async fn test_deploy(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.root;
    let alice = root
        .create_subaccount("alice", NearToken::from_near(100))
        .await;

    let state = OutlayerState::new(
        alice.account_id().clone(),
        [0u8; 32],
        EXAMPLE_URL.to_string(),
    );
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await;

    assert_eq!(instance.oa_admin_id().await.unwrap(), *alice.account_id());
    assert_eq!(instance.oa_code_hash().await.unwrap().0, [0u8; 32]);
    assert_eq!(
        instance.oa_code_url().await.unwrap(),
        EXAMPLE_URL.to_string()
    );
}

#[rstest]
#[tokio::test]
async fn test_deploy_with_pre_approved_hash(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.root;
    let code_hash = sha256_array(b"some-wasm-bytes");
    let state = OutlayerState::new(
        root.account_id().clone(),
        code_hash,
        EXAMPLE_URL.to_string(),
    );
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await;

    assert_eq!(instance.oa_code_hash().await.unwrap().0, code_hash);
}

#[rstest]
#[tokio::test]
async fn test_non_admin_cannot_set_code(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.root;
    let alice = root
        .create_subaccount("alice", NearToken::from_near(100))
        .await;

    let state = OutlayerState::new(
        alice.account_id().clone(),
        [0u8; 32],
        EXAMPLE_URL.to_string(),
    );
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await;

    let new_hash = [1u8; 32];
    root.oa_set_code(
        instance.contract_id(),
        [0u8; 32],
        new_hash,
        EXAMPLE_URL.to_string(),
    )
    .await
    .expect_err("non-admin should not be able to call oa_set_code");
}

#[rstest]
#[tokio::test]
async fn test_event_set_code(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.root;
    let state = OutlayerState::new(
        root.account_id().clone(),
        [0u8; 32],
        EXAMPLE_URL.to_string(),
    );
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await;

    let new_hash = [42u8; 32];
    let new_url = "https://new.example.com/contract.wasm".to_string();
    let result = root
        .oa_set_code(instance.contract_id(), [0u8; 32], new_hash, new_url.clone())
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::SetCode {
                hash: new_hash,
                url: new_url.into(),
            }
            .to_nep297_event()
            .to_event_log()
        ]
    );
}

#[rstest]
#[tokio::test]
async fn test_event_transfer_admin(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.root;
    let alice = root
        .create_subaccount("alice", NearToken::from_near(100))
        .await;

    let state = OutlayerState::new(
        root.account_id().clone(),
        [0u8; 32],
        EXAMPLE_URL.to_string(),
    );
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await;

    let result = root
        .oa_transfer_admin(instance.contract_id(), alice.account_id())
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::TransferAdmin {
                old_admin_id: root.account_id().into(),
                new_admin_id: alice.account_id().into(),
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );
}
