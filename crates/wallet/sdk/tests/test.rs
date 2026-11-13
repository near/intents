#![cfg(feature = "near-kit")]

use std::{env, fs, path::Path, sync::LazyLock};

use defuse_nep641::{JsonPayload, resolver::RpcResolver};
use defuse_wallet::{NearPromise, Request, WalletOp, actions::FunctionCall};
use defuse_wallet_ed25519::{WalletEd25519, WalletEd25519Signer, crypto::ed25519::ed25519_dalek};
use defuse_wallet_sdk::{
    Gas, NearToken, WalletBuilder,
    client::{WExecuteExtensionArgs, WExecuteSignedArgs, WalletContract},
};
use futures::{
    StreamExt, TryStreamExt, future,
    stream::{self, FuturesUnordered},
    try_join,
};
use near_kit::{
    CryptoHash, SigningNear,
    protocol::{Action, PublishMode},
    transaction::Final,
};
use near_kit_sandbox::Sandbox;
use rand::{rand_core::UnwrapErr, rngs::SysRng};
use rstest::{fixture, rstest};
use sha2::{Digest, Sha256};
use tokio::sync::OnceCell;
use tracing_subscriber::EnvFilter;

type Wallet = defuse_wallet_sdk::Wallet<WalletEd25519>;

#[rstest]
#[tokio::test]
#[awt]
async fn rotate(
    #[future] near: SigningNear,

    #[from(wallet)]
    #[future]
    master: Wallet,

    #[from(wallet)]
    #[future]
    extension: Wallet,
) {
    master
        .sign_and_send(
            Request::new()
                .internal([WalletOp::AddExtension {
                    account_id: extension.real_account_id().clone(),
                }])
                .external([NearPromise::new(extension.real_account_id())
                    .deterministic_state_init(
                        extension.deterministic_state_init().clone(),
                        NearToken::ZERO,
                    )
                    .function_call(
                        FunctionCall::name("w_execute_signed")
                            .attach_deposit(NearToken::from_yoctonear(1))
                            .args_json({
                                let (msg, proof) = extension
                                    .sign(
                                        NearPromise::new(master.real_account_id()).function_call(
                                            FunctionCall::name("w_execute_extension")
                                                .attach_deposit(NearToken::from_yoctonear(1))
                                                .args_json(WExecuteExtensionArgs::from(
                                                    Request::from(WalletOp::SetSignatureMode {
                                                        enable: false,
                                                    }),
                                                ))
                                                .gas(Gas::from_tgas(10)),
                                        ),
                                    )
                                    .await
                                    .unwrap();

                                WExecuteSignedArgs::from((msg, proof))
                            })
                            .gas(Gas::from_tgas(20)),
                    )]),
        )
        .await
        .unwrap()
        .status(near.near())
        .wait_until::<Final>()
        .await
        .unwrap()
        .result()
        .expect("key rotation failed");

    master
        .sign_and_send(Request::new())
        .await
        .unwrap()
        .status(near.near())
        .wait_until::<Final>()
        .await
        .unwrap()
        .result()
        .expect_err("signature should be disabled");

    let extension = extension.as_extension_of(master);

    extension
        .sign_and_send(Request::new())
        .await
        .unwrap()
        .status(near.near())
        .wait_until::<Final>()
        .await
        .unwrap()
        .result()
        .expect("extension should be able to execute requests on behalf of root");
}

#[rstest]
#[tokio::test]
#[awt]
async fn w_init(
    #[future] near: SigningNear,

    #[from(wallet)]
    #[future]
    extension: Wallet,

    #[from(wallet)]
    #[future]
    receiver: Wallet,
) {
    near.transaction(near.account_id())
        .add_action(Action::deploy_from(CryptoHash::from(
            *WALLET_NO_SIGN_CODE_HASH,
        )))
        .add_action(
            near_kit::transaction::FunctionCall::new("w_init")
                .gas(Gas::from_tgas(5))
                .deposit(NearToken::from_yoctonear(1)),
        )
        .await
        .unwrap()
        .result()
        .unwrap();

    near.contract::<WalletContract>(near.account_id())
        .w_execute_extension(
            Request::new()
                .internal([WalletOp::RemoveExtension {
                    account_id: near.account_id().into(),
                }])
                .into(),
        )
        .deposit(NearToken::from_yoctonear(1))
        .await
        .unwrap()
        .result()
        .expect_err("cannot accidentally delete itself from extension");

    near.contract::<WalletContract>(near.account_id())
        .w_execute_extension(
            Request::new()
                .internal([WalletOp::AddExtension {
                    account_id: extension.account_id().clone(),
                }])
                .into(),
        )
        .deposit(NearToken::from_yoctonear(1))
        .await
        .unwrap()
        .result()
        .unwrap();

    extension
        .as_extension_of(near.account_id())
        .sign_and_send(NearPromise::new(receiver.account_id()).transfer(NearToken::from_near(5)))
        .await
        .unwrap()
        .status(near.near())
        .wait_until::<Final>()
        .await
        .unwrap()
        .result()
        .unwrap();

    assert_eq!(
        near.near()
            .balance(receiver.account_id())
            .await
            .expect("implicit account should be created by incoming transfer")
            .total,
        NearToken::from_near(5),
    );
}

#[rstest]
#[case::empty("")]
#[case::test("test")]
#[case::json(JsonPayload {
    domain: "Near MPC".to_string(),
    action: "sign".to_string(),
    msg: "Hello, Near!".to_string(),
}.into())]
#[tokio::test]
#[awt]
async fn w_resolve_auth(
    #[case] payload: String,

    #[from(extension)]
    #[with(3)] // extensions depth
    #[future]
    wallet: Wallet,

    #[with(3)] // sub-authorizations depth
    #[future]
    resolver: RpcResolver,
) {
    let authorization = wallet
        .sign_offchain_msg(&payload, None) // top-level
        .await
        .expect("failed to sign");

    let resolved = resolver
        .resolve_auth(wallet.account_id(), &authorization)
        .await
        .expect("invalid authorization");

    println!("{authorization}\n{} -> {resolved}", wallet.account_id());
    assert_eq!(payload, resolved, "resolved invalid payload");
}

#[fixture]
#[awt]
async fn wallet(
    #[default(WalletBuilder::new())] builder: WalletBuilder,
    #[future] near: SigningNear,
) -> Wallet {
    builder
        .build(
            *WALLET_ED25519_CODE_HASH,
            WalletEd25519Signer(ed25519_dalek::SigningKey::generate(&mut UnwrapErr(SysRng))),
        )
        .with_client(near.near().clone())
        .with_relayer(near)
}

#[fixture]
#[awt]
async fn extension(
    #[default(1)] depth: usize,

    #[from(wallet)]
    #[future]
    ext: Wallet,

    #[future] near: SigningNear,
) -> Wallet {
    // recursively create chain of extensions
    let wallets = stream::unfold(ext, |ext| async {
        let w = wallet(
            WalletBuilder::new().extension(&ext),
            future::ready(near.clone()),
        )
        .await;
        Some((ext, w))
    })
    .take(depth + 1) // preserve original wallet
    .collect::<Vec<_>>()
    .await;

    // initialize all wallets
    wallets
        .iter()
        .map(Wallet::initialize)
        .collect::<FuturesUnordered<_>>()
        .try_collect::<()>()
        .await
        .unwrap();

    // reduce as extension chain
    wallets.into_iter().reduce(Wallet::as_extension_of).unwrap()
}

#[fixture]
#[awt]
async fn resolver(#[default(0)] depth: usize, #[future] near: SigningNear) -> RpcResolver {
    RpcResolver::new(near.near().rpc().clone())
        .await
        .expect("failed to initialize RPC resolver")
        .with_max_sub_authorizations(depth)
        .with_max_depth(depth)
}

#[fixture]
async fn near() -> SigningNear {
    static DEPLOY: OnceCell<()> = OnceCell::const_new();

    let near = Sandbox::builder()
        .shared()
        .await
        .expect("failed to start sandbox")
        .root_account();

    DEPLOY
        .get_or_init(|| async {
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::from_default_env())
                .pretty()
                .init();

            let (ed25519, no_sign) = try_join!(
                near.transaction(near.account_id())
                    .add_action(Action::publish(
                        &**WALLET_ED25519_WASM,
                        PublishMode::Immutable
                    ))
                    .into_future(),
                near.transaction(near.account_id())
                    .add_action(Action::publish(
                        &**WALLET_NO_SIGN_WASM,
                        PublishMode::Immutable
                    ))
                    .into_future(),
            )
            .expect("failed to deploy global contracts by hash");
            ed25519
                .result()
                .expect("failed to publish wallet-ed25519 by hash");
            no_sign
                .result()
                .expect("failed to publish wallet-no-sign by hash");
        })
        .await;

    near
}

static WALLET_ED25519_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse-wallet-ed25519.wasm"));
static WALLET_ED25519_CODE_HASH: LazyLock<[u8; 32]> =
    LazyLock::new(|| Sha256::digest(&*WALLET_ED25519_WASM).into());

static WALLET_NO_SIGN_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse-wallet-no-sign.wasm"));
static WALLET_NO_SIGN_CODE_HASH: LazyLock<[u8; 32]> =
    LazyLock::new(|| Sha256::digest(&*WALLET_NO_SIGN_WASM).into());

fn read_wasm(path: impl AsRef<Path>) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../")
        // if out dir path is absolute - base is ignored during join
        .join(env::var("DEFUSE_USE_OUT_DIR").as_deref().unwrap_or("./res"))
        .join(path);

    let path = fs::canonicalize(&path)
        .unwrap_or_else(|e| panic!("Failed to canonicalize path: {} {e}", path.display()));

    fs::read(&path)
        .unwrap_or_else(|e| panic!("Failed to read WASM file at {}: {e}", path.display()))
}
