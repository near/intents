#![cfg(feature = "near-kit")]

use std::{env, fs, path::Path, sync::LazyLock};

use defuse_wallet::{NearPromise, Request, WalletOp, actions::FunctionCall};
use defuse_wallet_ed25519::{WalletEd25519, WalletEd25519Signer, crypto::ed25519::ed25519_dalek};
use defuse_wallet_sdk::{
    Gas, NearToken,
    client::{WExecuteExtensionArgs, WExecuteSignedArgs, WalletContract},
};
use futures::try_join;
use near_kit::{CryptoHash, Final, Near, PublishMode, sandbox::SandboxConfig};
use rand::{rand_core::UnwrapErr, rngs::SysRng};
use rstest::{fixture, rstest};
use sha2::{Digest, Sha256};
use tokio::sync::OnceCell;

type Wallet = defuse_wallet_sdk::Wallet<WalletEd25519>;

#[rstest]
#[tokio::test]
#[awt]
async fn rotate(
    #[future] near: Near,
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
        .status(&near, Final)
        .await
        .unwrap()
        .result()
        .expect("key rotation failed");

    master
        .sign_and_send(Request::new())
        .await
        .unwrap()
        .status(&near, Final)
        .await
        .unwrap()
        .result()
        .expect_err("signature should be disabled");

    let extension = extension.as_extension_of(master);

    extension
        .sign_and_send(Request::new())
        .await
        .unwrap()
        .status(&near, Final)
        .await
        .unwrap()
        .result()
        .expect("extension should be able to execute requests on behalf of root");
}

#[rstest]
#[tokio::test]
#[awt]
async fn w_init(
    #[future] near: Near,
    #[from(wallet)]
    #[future]
    extension: Wallet,

    #[from(wallet)]
    #[future]
    receiver: Wallet,
) {
    near.deploy_from(CryptoHash::from(*WALLET_NO_SIGN_CODE_HASH))
        .add_action(
            near_kit::FunctionCall::new("w_init")
                .gas(Gas::from_tgas(5))
                .deposit(NearToken::from_yoctonear(1)),
        )
        .wait_until(Final)
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
        .status(&near, Final)
        .await
        .unwrap()
        .result()
        .unwrap();

    assert_eq!(
        near.balance(receiver.account_id())
            .await
            .expect("implicit account should be created by incoming transfer")
            .total,
        NearToken::from_near(5),
    );
}

#[fixture]
#[awt]
async fn wallet(#[future] near: Near) -> Wallet {
    Wallet::new(
        *WALLET_ED25519_CODE_HASH,
        WalletEd25519Signer(ed25519_dalek::SigningKey::generate(&mut UnwrapErr(SysRng))),
    )
    .with_client(near.clone())
    .with_relayer(near)
}

#[fixture]
async fn near() -> Near {
    static DEPLOY: OnceCell<()> = OnceCell::const_new();

    let near = SandboxConfig::shared().await.client();

    DEPLOY
        .get_or_init(|| async {
            try_join!(
                near.publish(&**WALLET_ED25519_WASM, PublishMode::Immutable)
                    .into_future(),
                near.publish(&**WALLET_NO_SIGN_WASM, PublishMode::Immutable)
                    .into_future(),
            )
            .expect("failed to deploy global contracts by hash");
        })
        .await;

    near
}

static WALLET_ED25519_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let wasm = Path::new(env::var("DEFUSE_USE_OUT_DIR").as_deref().unwrap_or("./res"))
        .join("defuse-wallet-ed25519.wasm");
    fs::read(wasm).expect("failed to read WASM")
});
static WALLET_ED25519_CODE_HASH: LazyLock<[u8; 32]> =
    LazyLock::new(|| Sha256::digest(&*WALLET_ED25519_WASM).into());

static WALLET_NO_SIGN_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let wasm = Path::new(env::var("DEFUSE_USE_OUT_DIR").as_deref().unwrap_or("./res"))
        .join("defuse-wallet-no-sign.wasm");
    fs::read(wasm).expect("failed to read WASM")
});
static WALLET_NO_SIGN_CODE_HASH: LazyLock<[u8; 32]> =
    LazyLock::new(|| Sha256::digest(&*WALLET_NO_SIGN_WASM).into());
