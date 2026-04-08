use defuse_outlayer_app::{Event, State as OutlayerState, Url};
use defuse_sandbox::{
    Sandbox,
    api::types::transaction::actions::GlobalContractDeployMode,
    extensions::outlayer_app::{OutlayerAppExt, OutlayerAppViewExt},
    sandbox,
};
use defuse_test_utils::wasms::OUTLAYER_APP_WASM;
use near_sdk::{AsNep297Event, GlobalContractId, NearToken, env::sha256_array};
use rstest::{fixture, rstest};

const EXAMPLE_URL: &str = "https://example.com/contract.wasm";

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

fn example_url() -> Url {
    Url::parse(EXAMPLE_URL).unwrap()
}

#[rstest]
#[tokio::test]
async fn test_deploy(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    let code_url = example_url();
    let state = OutlayerState::new(alice.id().clone(), [0u8; 32], code_url.clone());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    assert_eq!(instance.oa_admin_id().await.unwrap(), *alice.id());
    assert_eq!(instance.oa_code_hash().await.unwrap(), [0u8; 32]);
    assert_eq!(instance.oa_code_url().await.unwrap(), code_url);
}

#[rstest]
#[tokio::test]
async fn test_deploy_with_pre_approved_hash(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let code_hash = sha256_array(b"some-wasm-bytes");
    let state = OutlayerState::new(root.id().clone(), code_hash, example_url());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    assert_eq!(instance.oa_code_hash().await.unwrap(), code_hash);
}

#[rstest]
#[tokio::test]
async fn test_non_admin_cannot_set_code(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    let state = OutlayerState::new(alice.id().clone(), [0u8; 32], example_url());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let new_hash = [1u8; 32];
    root.oa_set_code(instance.id(), new_hash, example_url())
        .await
        .expect_err("non-admin should not be able to call oa_set_code");
}

#[rstest]
#[tokio::test]
async fn test_event_set_code(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let state = OutlayerState::new(root.id().clone(), [0u8; 32], example_url());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let new_hash = [42u8; 32];
    let new_url = Url::parse("https://new.example.com/contract.wasm").unwrap();
    let result = root
        .oa_set_code(instance.id(), new_hash, new_url.clone())
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::SetCode {
                hash: new_hash,
                url: new_url,
            }
            .to_nep297_event()
            .to_event_log()
        ]
    );
}

#[rstest]
#[tokio::test]
async fn test_event_transfer_admin(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();
    let state = OutlayerState::new(root.id().clone(), [0u8; 32], example_url());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let result = root
        .oa_transfer_admin(instance.id(), alice.id())
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::TransferAdmin {
                old_admin_id: root.id().into(),
                new_admin_id: alice.id().into(),
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );
}
