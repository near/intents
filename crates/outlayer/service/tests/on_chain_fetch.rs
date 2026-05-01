use std::{borrow::Cow, sync::Arc};

use bytes::Bytes;
use defuse_outlayer_app::State as OutlayerState;
use defuse_outlayer_crypto::signer::InMemorySigner as SigningSigner;
use defuse_outlayer_host::{
    Context, InMemorySigner as HostSigner, State as HostState,
    primitives::{AccountIdRef, AppId},
};
use defuse_outlayer_service::{
    Config, OnChainFetchService, Request, build_stack,
    types::{AccountId, OffChainRequest},
};
use defuse_outlayer_vm_runner::VmRuntime;
use defuse_sandbox::{
    Sandbox, api::types::transaction::actions::GlobalContractDeployMode,
    extensions::outlayer_app::OutlayerAppExt as _, sandbox,
};
use defuse_test_utils::wasms::OUTLAYER_APP_WASM;
use near_sdk::{GlobalContractId, env::sha256_array};
use rstest::rstest;
use sha2::{Digest as _, Sha256};
use std::future::IntoFuture as _;

use axum::{Router, routing::get};
use tower::ServiceExt as _;

async fn spawn_wasm_server(wasm_bytes: Arc<Vec<u8>>) -> std::net::SocketAddr {
    let wasm = Bytes::copy_from_slice(&wasm_bytes);
    let app = Router::new().route(
        "/near-pds.wasm",
        get(move || async move {
            (
                [(axum::http::header::CONTENT_TYPE, "application/wasm")],
                wasm,
            )
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app).into_future());
    addr
}

#[rstest]
#[tokio::test]
#[allow(clippy::future_not_send)]
async fn test_on_chain_fetch_service(#[future(awt)] sandbox: Sandbox) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .from_env_lossy()
                .add_directive("defuse_outlayer_service=debug".parse().unwrap()),
        )
        .try_init();

    let wasm_bytes = Arc::new(
        std::fs::read(std::path::Path::new(env!("HOME")).join("near-pds.wasm"))
            .expect("~/near-pds.wasm not found"),
    );
    let wasm_hash: [u8; 32] = Sha256::digest(&**wasm_bytes).into();

    let addr = spawn_wasm_server(Arc::clone(&wasm_bytes)).await;
    let wasm_url = format!("http://{addr}/near-pds.wasm");

    let root = sandbox.root();
    root.deploy_global_contract(
        OUTLAYER_APP_WASM.clone(),
        GlobalContractDeployMode::CodeHash,
    )
    .await
    .unwrap();
    let global_id = GlobalContractId::CodeHash(sha256_array(&*OUTLAYER_APP_WASM).into());

    let state = OutlayerState::new(root.id().clone(), wasm_hash, wasm_url);
    let instance = root.deploy_outlayer_app(global_id, state).await.unwrap();

    let signing_key = SigningSigner::from_seed(&[1u8; 32]);
    let host_template = HostState::new(
        Context {
            app_id: AppId::Near(Cow::Borrowed(AccountIdRef::new_or_panic("test.near"))),
        },
        Cow::Owned(HostSigner::from_seed(b"test")),
    );
    let runtime = Arc::new(VmRuntime::<HostState<'static>>::new().unwrap());
    let fetch = OnChainFetchService::with_network_config(root.network_config().clone());

    let response = build_stack(
        signing_key,
        runtime,
        Config::default(),
        host_template,
        fetch,
    )
    .oneshot(Request::OffChain(OffChainRequest {
        request_id: "test".to_string(),
        project_id: AccountId(instance.id().to_string()),
        input: Bytes::new(),
    }))
    .await
    .unwrap();

    assert!(response.response.result.is_ok());
}
