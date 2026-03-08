#![allow(clippy::future_not_send)]
use std::time::Duration;

use defuse_randomness::{rng, rngs::ThreadRng};
use defuse_sandbox::{
    Sandbox,
    extensions::wallet::{WalletExt, WalletViewExt},
    sandbox,
};
use defuse_test_utils::wasms::WALLET_WASM;
use defuse_wallet::{
    self, ConcurrentNonces, FunctionCallAction, PromiseSingle, Request, State, WalletOp,
    signature::{
        Borsh, Deadline, RequestMessage, SigningStandard, WALLET_DOMAIN,
        ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
    },
};
use futures::{TryStreamExt, stream::FuturesUnordered};
use impl_tools::autoimpl;
use near_crypto::{KeyType, SecretKey, Signature};
use near_sdk::{
    AccountId, Gas, GlobalContractId, NearToken, borsh,
    env::sha256_array,
    state_init::{StateInit, StateInitV1},
};
use rstest::{fixture, rstest};
use serde_json::json;

type S = Borsh<Ed25519>;
type PublicKey = <S as SigningStandard<RequestMessage>>::PublicKey;

#[rstest]
#[awt]
#[tokio::test]
async fn test_signed(#[future] env: Env) {
    let mut wallet = env.generate_wallet();
    let wallet_account = env.account(wallet.id());

    let receiver = env
        .generate_subaccount("receiver", NearToken::ZERO)
        .await
        .unwrap();

    let (msg, proof) = wallet.sign(
        Request::new()
            .ops([
                WalletOp::AddExtension {
                    account_id: env.root().id().clone(),
                },
                WalletOp::RemoveExtension {
                    account_id: env.root().id().clone(),
                },
            ])
            .out(
                PromiseSingle::new(receiver.id())
                    .transfer(NearToken::from_yoctonear(1))
                    .then(PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(2)))
                    .and(PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(3)))
                    .then_concurrent([
                        PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(4)),
                        PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(5)),
                    ])
                    .then(PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(6))),
            ),
    );

    env.w_execute_signed(
        wallet.id(),
        wallet.state_init(),
        msg.clone(),
        proof.clone(),
        NearToken::from_near(1),
    )
    .await
    .unwrap();

    env.w_execute_signed(
        wallet.id(),
        wallet.state_init(),
        msg,
        proof,
        NearToken::from_near(1),
    )
    .await
    .expect_err("nonce should be already used");

    assert!(wallet_account.view().await.unwrap().amount >= NearToken::from_near(1));
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_rotate(#[future] env: Env) {
    let [mut old_wallet, mut new_wallet] = [env.generate_wallet(), env.generate_wallet()];

    let [old_wallet_account, _new_wallet_account] =
        [old_wallet.id(), new_wallet.id()].map(|account_id| env.account(account_id));

    let (msg, proof) = old_wallet.sign(
        Request::new()
            .ops([WalletOp::AddExtension {
                account_id: new_wallet.id(),
            }])
            .out(
                PromiseSingle::new(new_wallet.id())
                    .state_init(new_wallet.state_init(), NearToken::ZERO)
                    .function_call(
                        FunctionCallAction::new("w_execute_signed")
                            .attached_deposit(NearToken::from_yoctonear(1))
                            .min_gas(Gas::from_tgas(20))
                            .args_json({
                                let (msg, proof) = new_wallet.sign(
                                    Request::new().out(
                                        PromiseSingle::new(old_wallet.id()).function_call(
                                            FunctionCallAction::new("w_execute_extension")
                                                .attached_deposit(NearToken::from_yoctonear(1))
                                                .min_gas(Gas::from_tgas(10))
                                                .args_json(json!({
                                                    "request": Request::new().ops([
                                                        WalletOp::SetSignatureMode { enable: false }
                                                    ])
                                                })),
                                        ),
                                    ),
                                );
                                json!({
                                    "msg": msg,
                                    "proof": proof,
                                })
                            }),
                    ),
            ),
    );

    env.w_execute_signed(
        old_wallet.id(),
        old_wallet.state_init(),
        msg,
        proof,
        NearToken::from_yoctonear(1),
    )
    .await
    .unwrap();

    assert!(!old_wallet_account.w_is_signature_allowed().await.unwrap());

    {
        let (msg, proof) = old_wallet.sign(Request::default());
        env.w_execute_signed(old_wallet.id(), None, msg, proof, NearToken::ZERO)
            .await
            .expect_err("signature should be disabled");
    }

    let (msg, proof) = new_wallet.sign(
        Request::new().out(
            PromiseSingle::new(old_wallet.id()).function_call(
                FunctionCallAction::new("w_execute_extension")
                    .attached_deposit(NearToken::from_yoctonear(1))
                    .args_json(json!({
                        "request": Request::new(),
                    })),
            ),
        ),
    );

    env.w_execute_signed(new_wallet.id(), None, msg, proof, NearToken::ZERO)
        .await
        .unwrap();
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_extension(#[future] env: Env) {
    let extension = env
        .generate_subaccount("extension", NearToken::from_near(100))
        .await
        .unwrap();

    let wallet_state_init = StateInit::V1(StateInitV1 {
        code: env.wallet_global_id.clone(),
        data: State::<PublicKey>::new(Ed25519PublicKey([0; 32]))
            .extensions([extension.id()])
            .as_storage(),
    });

    // 0s123445
    let wallet = env.account(wallet_state_init.derive_account_id());

    let receiver = env
        .generate_subaccount("receiver", NearToken::ZERO)
        .await
        .unwrap();

    let refund_to = env
        .generate_subaccount("refund_to", NearToken::ZERO)
        .await
        .unwrap();

    extension
        .w_execute_extension(
            wallet.id(),
            wallet_state_init.clone(),
            Request::new()
                .ops([WalletOp::RemoveExtension {
                    account_id: extension.id().clone(),
                }])
                .out(
                    PromiseSingle::new(receiver.id())
                        .refund_to(refund_to.id())
                        .transfer(NearToken::from_near(1)),
                ),
            NearToken::from_near(1),
        )
        .await
        .unwrap();

    assert!(receiver.view().await.unwrap().amount >= NearToken::from_near(1));
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_zba(#[future] env: Env) {
    let mut wallet = env.generate_wallet();
    let wallet_account = env.account(wallet.id());

    let wallet_id = wallet.id();
    let wallet_state_init = wallet.state_init();

    (0..wallet.init_state.nonces.timeout().as_secs() * 2)
        .map(|_n| wallet.sign(Request::new()))
        .map(|(msg, proof)| {
            let env = &env;
            let wallet_id = wallet_id.clone();
            let wallet_state_init = wallet_state_init.clone();
            async move {
                env.w_execute_signed(wallet_id, wallet_state_init, msg, proof, NearToken::ZERO)
                    .await
                    .map(|_| ())
            }
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<()>()
        .await
        .unwrap();

    dbg!(wallet_account.view().await.unwrap());
}

#[derive(Debug)]
pub struct WalletEd25519 {
    global_contract_id: GlobalContractId,

    init_state: State<PublicKey>,

    secret_key: SecretKey,

    nonces: ConcurrentNonces<ThreadRng>,
}

impl WalletEd25519 {
    pub fn new(global_contract_id: GlobalContractId, secret_key: SecretKey) -> Self {
        Self {
            global_contract_id,
            init_state: State::new(Ed25519PublicKey(
                secret_key.public_key().unwrap_as_ed25519().0,
            )),
            secret_key,
            nonces: ConcurrentNonces::new(rng()),
        }
    }

    pub fn generate(global_contract_id: GlobalContractId) -> Self {
        Self::new(global_contract_id, SecretKey::from_random(KeyType::ED25519))
    }

    pub fn state_init(&self) -> StateInit {
        StateInit::V1(StateInitV1 {
            code: self.global_contract_id.clone(),
            data: self.init_state.as_storage(),
        })
    }

    pub fn id(&self) -> AccountId {
        self.state_init().derive_account_id()
    }

    pub fn sign(&mut self, request: Request) -> (RequestMessage, String) {
        let msg = RequestMessage {
            chain_id: "mainnet".to_string(),
            signer_id: self.id(),
            nonce: self.nonces.next().unwrap(),
            created_at: Deadline::now() - Duration::from_secs(60),
            timeout: self.init_state.nonces.timeout(),
            request,
        };

        let hash = sha256_array([WALLET_DOMAIN, &borsh::to_vec(&msg).unwrap()].concat());
        let Signature::ED25519(signature) = self.secret_key.sign(&hash) else {
            unreachable!("invalid signature type");
        };
        (msg, Ed25519Signature(signature.to_bytes()).to_string())
    }
}

#[autoimpl(Deref using self.sandbox)]
struct Env {
    pub wallet_global_id: GlobalContractId,

    sandbox: Sandbox,
}

impl Env {
    pub fn generate_wallet(&self) -> WalletEd25519 {
        WalletEd25519::generate(self.wallet_global_id.clone())
    }
}

#[fixture]
#[awt]
async fn env(#[future] sandbox: Sandbox) -> Env {
    // wallet.0.test
    let wallet_contract = sandbox
        .root()
        .deploy_global_sub_contract("wallet", NearToken::from_near(1000), WALLET_WASM.clone())
        .await
        .unwrap();

    Env {
        wallet_global_id: wallet_contract.id().clone().into(),
        sandbox,
    }
}
