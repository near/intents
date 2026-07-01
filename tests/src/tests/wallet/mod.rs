mod no_sign;

use std::borrow::Cow;

use defuse_sandbox::{
    account::Account,
    extensions::wallet::{
        WExecuteExtensionArgs, WExecuteSignedArgs, Wallet, WalletExt,
        sdk::{
            NearPromise, Request, State, WalletOp, WalletSigner,
            actions::FunctionCall,
            ed25519::ed25519_dalek::{self, ed25519::signature::rand_core::OsRng},
        },
    },
    global_contract::GlobalContract,
    kit::{Finality::Optimistic, Gas, GlobalContractId, Near, NearToken, StateInit, StateInitV1},
    root,
};
use defuse_test_utils::wasms::WALLET_WASM;
use defuse_wallet_relayer::{WalletRelayRequest, WalletRelayer};
use futures::future::join_all;
use impl_tools::autoimpl;
use rstest::{fixture, rstest};

#[rstest]
#[awt]
#[tokio::test]
async fn test_signed(#[future] env: Env) {
    let mut wallet = env.generate_wallet();

    let receiver = env.create_subaccount("receiver", NearToken::ZERO).await;

    let (msg, proof) = wallet
        .sign(
            Request::new()
                .internal([
                    WalletOp::AddExtension {
                        account_id: env.account_id().clone(),
                    },
                    WalletOp::RemoveExtension {
                        account_id: env.account_id().clone(),
                    },
                ])
                .external([
                    NearPromise::new(receiver.account_id()).transfer(NearToken::from_yoctonear(1)),
                    NearPromise::new(receiver.account_id()).transfer(NearToken::from_yoctonear(2)),
                    NearPromise::new(receiver.account_id()).transfer(NearToken::from_yoctonear(3)),
                ]),
        )
        .unwrap();

    let request =
        WalletRelayRequest::new(msg.clone(), &proof).state_init(wallet.deterministic_state_init());

    assert!(
        env.relayer
            .w_execute_signed(request.clone(), NearToken::from_near(1), None)
            .await
            .unwrap()
            .is_success(),
    );

    assert!(
        env.relayer
            .w_execute_signed(request.clone(), NearToken::from_near(1), None)
            .await
            .unwrap()
            .is_failure(),
        "nonce should be already used"
    );

    assert!(
        env.account(wallet.account_id())
            .finality(Optimistic)
            .await
            .unwrap()
            .amount
            >= NearToken::from_near(1)
    );
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_rotate(#[future] env: Env) {
    let [mut old_wallet, mut new_wallet] = [env.generate_wallet(), env.generate_wallet()];

    let (msg, proof) = old_wallet
        .sign(
            Request::new()
                .internal([WalletOp::AddExtension {
                    account_id: new_wallet.account_id().clone(),
                }])
                .external([NearPromise::new(new_wallet.account_id())
                    .deterministic_state_init(
                        new_wallet.deterministic_state_init(),
                        NearToken::ZERO,
                    )
                    .function_call(
                        FunctionCall::name("w_execute_signed")
                            .attach_deposit(NearToken::from_yoctonear(1))
                            .args_json({
                                let (msg, proof) = new_wallet
                                    .sign(
                                        Request::new().external([NearPromise::new(
                                            old_wallet.account_id(),
                                        )
                                        .function_call(
                                            FunctionCall::name("w_execute_extension")
                                                .attach_deposit(NearToken::from_yoctonear(1))
                                                .args_json(WExecuteExtensionArgs {
                                                    request: Cow::Owned(Request::new().internal([
                                                        WalletOp::SetSignatureMode {
                                                            enable: false,
                                                        },
                                                    ])),
                                                })
                                                .gas(Gas::from_tgas(10)),
                                        )]),
                                    )
                                    .unwrap();

                                WExecuteSignedArgs {
                                    msg: Cow::Owned(msg),
                                    proof: proof.into(),
                                }
                            })
                            .gas(Gas::from_tgas(20)),
                    )]),
        )
        .unwrap();

    assert!(
        env.relayer
            .w_execute_signed(
                WalletRelayRequest::new(msg, proof)
                    .state_init(old_wallet.deterministic_state_init()),
                NearToken::from_yoctonear(1),
                None,
            )
            .await
            .unwrap()
            .is_success()
    );

    assert!(
        !env.contract::<Wallet>(old_wallet.account_id())
            .w_is_signature_allowed()
            .finality(Optimistic)
            .await
            .unwrap()
    );

    {
        let (msg, proof) = old_wallet.sign(Request::default()).unwrap();

        assert!(
            env.relayer
                .w_execute_signed(WalletRelayRequest::new(msg, proof), NearToken::ZERO, None)
                .await
                .unwrap()
                .is_failure(),
            "signature should be disabled",
        );
    }

    let (msg, proof) = new_wallet
        .sign(
            Request::new().external([NearPromise::new(old_wallet.account_id()).function_call(
                FunctionCall::name("w_execute_extension")
                    .attach_deposit(NearToken::from_yoctonear(1))
                    .args_json(WExecuteExtensionArgs {
                        request: Cow::Owned(Request::new()),
                    })
                    .gas(Gas::from_tgas(10)),
            )]),
        )
        .unwrap();

    assert!(
        env.relayer
            .w_execute_signed(WalletRelayRequest::new(msg, proof), NearToken::ZERO, None)
            .await
            .unwrap()
            .is_success()
    );
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
        data: State::<[u8; 32]>::default()
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
            &Request::new()
                .internal([WalletOp::RemoveExtension {
                    account_id: extension.account_id().clone(),
                }])
                .external([NearPromise::new(receiver.account_id())
                    .refund_to(refund_to.account_id())
                    .transfer(NearToken::from_near(1))]),
            NearToken::from_near(1),
        )
        .await
        .unwrap();

    assert!(
        env.account(receiver.account_id())
            .finality(Optimistic)
            .await
            .unwrap()
            .amount
            >= NearToken::from_near(1)
    );
}

#[rstest]
#[awt]
#[cfg_attr(not(feature = "long"), ignore = "`long` feature is disabled")]
#[tokio::test]
async fn test_no_storage_staking(#[future] env: Env) {
    let mut wallet = env.generate_wallet();

    let wallet_id = wallet.account_id().clone();
    let wallet_state_init = wallet.deterministic_state_init();

    // do state_init in advance
    env.transaction(wallet_id.clone())
        .state_init(wallet_state_init, NearToken::ZERO)
        .await
        .unwrap()
        .result()
        .unwrap();

    join_all(
        (0..wallet.nonces.timeout().as_secs() * 2)
            .map(|_n| {
                let (msg, proof) = wallet.sign(Request::new()).unwrap();
                WalletRelayRequest::new(msg, proof)
            })
            .map(|req| async {
                assert!(
                    env.relayer
                        .w_execute_signed(req, NearToken::ZERO, None)
                        .await
                        .unwrap()
                        .is_success()
                );
            }),
    )
    .await;
}

#[autoimpl(Deref using self.root)]
struct Env {
    pub wallet_global_id: GlobalContractId,

    pub relayer: WalletRelayer,

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
async fn env(
    #[default(WALLET_WASM.clone())] wasm: impl Into<Vec<u8>>,
    #[future] root: Near,
) -> Env {
    // wallet.0.test
    let wallet_global_id = root
        .deploy_upgradable_global_contract(
            root.account_id().sub_account("wallet").unwrap(),
            wasm,
            NearToken::from_near(1000),
        )
        .await
        .unwrap();

    Env {
        wallet_global_id,
        relayer: WalletRelayer::new(root.clone()),
        root,
    }
}
