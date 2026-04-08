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
    let state = OutlayerState::new(alice.id().clone(), code_url.clone());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    assert_eq!(instance.op_admin_id().await.unwrap(), *alice.id());
    assert_eq!(
        instance.op_code_hash().await.unwrap(),
        OutlayerState::DEFAULT_HASH
    );
    assert_eq!(instance.op_code_uri().await.unwrap(), code_url);
}

#[rstest]
#[tokio::test]
async fn test_deploy_with_pre_approve(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let code_hash = sha256_array(b"some-wasm-bytes");
    let state = OutlayerState::new(root.id().clone(), example_url()).pre_approve(code_hash);
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    assert_eq!(instance.op_code_hash().await.unwrap(), code_hash);
}

#[rstest]
#[tokio::test]
async fn test_non_admin_cannot_approve(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();

    let state = OutlayerState::new(alice.id().clone(), example_url());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let new_hash = [1u8; 32];
    root.op_approve(instance.id(), new_hash)
        .await
        .expect_err("non-admin should not be able to call op_approve");
}

#[rstest]
#[tokio::test]
async fn test_event_approve(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let state = OutlayerState::new(root.id().clone(), example_url());
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
async fn test_event_set_admin_id(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();
    let state = OutlayerState::new(root.id().clone(), example_url());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let result = root
        .op_set_admin_id(instance.id(), alice.id())
        .await
        .unwrap();

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
async fn test_event_set_code_uri(#[future(awt)] outlayer_app_env: OutlayerAppEnv) {
    let root = outlayer_app_env.sandbox.root();
    let state = OutlayerState::new(root.id().clone(), example_url());
    let instance = root
        .deploy_outlayer_app(outlayer_app_env.global_id.clone(), state)
        .await
        .unwrap();

    let new_url = Url::parse("https://new.example.com/contract.wasm").unwrap();
    let result = root
        .op_set_code_uri(instance.id(), new_url.clone())
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::SetCodeUri { url: new_url }
                .to_nep297_event()
                .to_event_log()
        ]
    );
}
