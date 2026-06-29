use defuse_sandbox::{
    account::Account,
    extensions::wallet::{
        Wallet, WalletExt,
        contract::{
            Request, State, WalletOp,
            promise::{
                NearPromise,
                actions::{FunctionCall, StateInitAction},
            },
            signature::ed25519::Ed25519PublicKey,
        },
        sdk::{
            WalletSigner,
            ed25519::ed25519_dalek::{self, ed25519::signature::rand_core::OsRng},
        },
    },
    global_contract::GlobalContract,
    kit::{Gas, GlobalContractId, Near, NearToken, StateInit, StateInitV1},
    root,
};
use defuse_test_utils::wasms::WALLET_WASM;
use futures::{TryStreamExt, stream::FuturesUnordered};
use impl_tools::autoimpl;
use rstest::{fixture, rstest};
use serde_json::json;

#[rstest]
#[awt]
#[tokio::test]
async fn test_signed(#[future] env: Env) {
    let mut wallet = env.generate_wallet();

    let receiver = env.create_subaccount("receiver", NearToken::ZERO).await;

    let (msg, proof) = wallet
        .sign(
            Request::new()
                .ops([
                    WalletOp::AddExtension {
                        account_id: env.account_id().clone(),
                    },
                    WalletOp::RemoveExtension {
                        account_id: env.account_id().clone(),
                    },
                ])
                .out([
                    NearPromise::new(receiver.account_id()).transfer(NearToken::from_yoctonear(1)),
                    NearPromise::new(receiver.account_id()).transfer(NearToken::from_yoctonear(2)),
                    NearPromise::new(receiver.account_id()).transfer(NearToken::from_yoctonear(3)),
                ]),
        )
        .unwrap();

    env.w_execute_signed(
        wallet.account_id(),
        Some(wallet.state_init()),
        msg.clone(),
        proof.clone(),
        NearToken::from_near(1),
    )
    .await
    .unwrap();

    env.w_execute_signed(
        wallet.account_id(),
        Some(wallet.state_init()),
        msg,
        proof,
        NearToken::from_near(1),
    )
    .await
    .expect_err("nonce should be already used");

    assert!(env.account(wallet.account_id()).await.unwrap().amount >= NearToken::from_near(1));
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_rotate(#[future] env: Env) {
    let [mut old_wallet, mut new_wallet] = [env.generate_wallet(), env.generate_wallet()];

    let (msg, proof) = old_wallet
        .sign(
            Request::new()
                .ops([WalletOp::AddExtension {
                    account_id: new_wallet.account_id().clone(),
                }])
                .out([NearPromise::new(new_wallet.account_id())
                    .add_action(StateInitAction::legacy(&new_wallet.state_init()))
                    .function_call(
                        FunctionCall::name("w_execute_signed")
                            .attach_deposit(NearToken::from_yoctonear(1))
                            .args_json({
                                let (msg, proof) = new_wallet
                                    .sign(
                                        Request::new().out([NearPromise::new(
                                            old_wallet.account_id(),
                                        )
                                        .function_call(
                                            FunctionCall::name("w_execute_extension")
                                                .attach_deposit(NearToken::from_yoctonear(1))
                                                .args_json(json!({
                                                    "request": Request::new().ops([
                                                        WalletOp::SetSignatureMode { enable: false }
                                                    ])
                                                }))
                                                .gas(Gas::from_tgas(10)),
                                        )]),
                                    )
                                    .unwrap();

                                json!({
                                    "msg": msg,
                                    "proof": proof,
                                })
                            })
                            .gas(Gas::from_tgas(20)),
                    )]),
        )
        .unwrap();

    env.w_execute_signed(
        old_wallet.account_id(),
        old_wallet.state_init(),
        msg,
        proof,
        NearToken::from_yoctonear(1),
    )
    .await
    .unwrap();

    assert!(
        !env.contract::<Wallet>(old_wallet.account_id())
            .w_is_signature_allowed()
            .await
            .unwrap()
    );

    {
        let (msg, proof) = old_wallet.sign(Request::default()).unwrap();
        env.w_execute_signed(old_wallet.account_id(), None, msg, proof, NearToken::ZERO)
            .await
            .expect_err("signature should be disabled");
    }

    let (msg, proof) = new_wallet
        .sign(
            Request::new().out([NearPromise::new(old_wallet.account_id()).function_call(
                FunctionCall::name("w_execute_extension")
                    .attach_deposit(NearToken::from_yoctonear(1))
                    .args_json(json!({
                        "request": Request::new(),
                    }))
                    .gas(Gas::from_tgas(10)),
            )]),
        )
        .unwrap();

    env.w_execute_signed(new_wallet.account_id(), None, msg, proof, NearToken::ZERO)
        .await
        .unwrap();
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_extension(#[future] env: Env) {
    let extension = env
        .create_subaccount("extension", NearToken::from_near(100))
        .await;

    let wallet_state_init = StateInit::V1(StateInitV1 {
        code: env.wallet_global_id.clone(),
        data: State::new(Ed25519PublicKey([0; 32]))
            .extensions([extension.account_id()])
            .as_storage(),
    });

    // 0s123445
    let receiver = env.create_subaccount("receiver", NearToken::ZERO).await;
    let refund_to = env.create_subaccount("refund_to", NearToken::ZERO).await;

    extension
        .w_execute_extension(
            wallet_state_init.derive_account_id(),
            wallet_state_init.clone(),
            Request::new()
                .ops([WalletOp::RemoveExtension {
                    account_id: extension.account_id().clone(),
                }])
                .out([NearPromise::new(receiver.account_id())
                    .refund_to(refund_to.account_id())
                    .transfer(NearToken::from_near(1))]),
            NearToken::from_near(1),
        )
        .await
        .unwrap();

    assert!(env.account(receiver.account_id()).await.unwrap().amount >= NearToken::from_near(1));
}

#[rstest]
#[awt]
#[cfg_attr(not(feature = "long"), ignore = "`long` feature is disabled")]
#[tokio::test]
async fn test_no_storage_staking(#[future] env: Env) {
    let mut wallet = env.generate_wallet();

    let wallet_id = wallet.account_id().clone();
    let wallet_state_init = wallet.state_init();

    // do state_init in advance
    env.transaction(wallet_id.clone())
        .state_init(wallet_state_init, NearToken::ZERO)
        .await
        .unwrap()
        .result()
        .unwrap();

    (0..wallet.nonces.timeout().as_secs() * 2)
        .map(|_n| wallet.sign(Request::new()).unwrap())
        .map(|(msg, proof)| {
            let env = &env;
            let wallet_id = wallet_id.clone();
            async move {
                env.w_execute_signed(wallet_id, None, msg, proof, NearToken::ZERO)
                    .await
                    .map(|_| ())
            }
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<()>()
        .await
        .unwrap();
}

#[autoimpl(Deref using self.root)]
struct Env {
    pub wallet_global_id: GlobalContractId,

    root: Near,
}

impl Env {
    pub fn generate_wallet(&self) -> WalletSigner<ed25519_dalek::SigningKey> {
        WalletSigner::new(
            self.wallet_global_id.clone(),
            ed25519_dalek::SigningKey::generate(&mut OsRng),
        )
    }
}

#[fixture]
#[awt]
async fn env(#[future] root: Near) -> Env {
    // wallet.0.test
    let wallet_global_id = root
        .deploy_upgradable_global_contract(
            root.account_id().sub_account("wallet").unwrap(),
            WALLET_WASM.clone(),
            NearToken::from_near(1000),
        )
        .await
        .unwrap();

    Env {
        wallet_global_id,
        root,
    }
}
