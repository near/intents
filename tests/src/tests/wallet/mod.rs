use std::time::Duration;

use defuse_randomness::rng;
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
    Gas, GlobalContractId, NearToken, borsh,
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
    let secret_key = SecretKey::from_random(KeyType::ED25519);

    let wallet_state = State::<PublicKey>::new(Ed25519PublicKey(
        secret_key.public_key().unwrap_as_ed25519().0,
    ));

    let wallet_state_init = StateInit::V1(StateInitV1 {
        code: env.wallet_global_id.clone(),
        data: wallet_state.as_storage(),
    });

    let wallet = env.account(wallet_state_init.derive_account_id());
    let mut nonces = ConcurrentNonces::new(rng());

    let receiver = env
        .generate_subaccount("receiver", NearToken::ZERO)
        .await
        .unwrap();

    let request = Request {
        ops: vec![
            WalletOp::AddExtension {
                account_id: env.root().id().clone(),
            },
            WalletOp::RemoveExtension {
                account_id: env.root().id().clone(),
            },
        ],
        out: dbg!(
            PromiseSingle::new(receiver.id())
                .transfer(NearToken::from_yoctonear(1))
                .then(PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(2)))
                .and(PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(3)))
                .then_concurrent([
                    PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(4)),
                    PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(5))
                ])
                .then(PromiseSingle::new(receiver.id()).transfer(NearToken::from_yoctonear(6)))
        ),
    };

    let signed_request_body = RequestMessage {
        chain_id: "mainnet".to_string(),
        signer_id: wallet.id().clone(),
        nonce: nonces.next().unwrap(),
        created_at: Deadline::now() - Duration::from_secs(60),
        timeout: wallet_state.nonces.timeout(),
        request,
    };

    env.w_execute_signed(
        wallet.id(),
        wallet_state_init.clone(),
        signed_request_body.clone(),
        sign_request(&secret_key, &signed_request_body),
        NearToken::from_near(1),
    )
    .await
    .unwrap();

    env.w_execute_signed(
        wallet.id(),
        wallet_state_init.clone(),
        signed_request_body.clone(),
        sign_request(&secret_key, &signed_request_body),
        NearToken::from_near(1),
    )
    .await
    .expect_err("nonce should be already used");

    assert!(wallet.view().await.unwrap().amount >= NearToken::from_near(1));
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
            Request {
                ops: vec![WalletOp::RemoveExtension {
                    account_id: extension.id().clone(),
                }],
                out: PromiseSingle::new(receiver.id())
                    .refund_to(refund_to.id())
                    .transfer(NearToken::from_near(1))
                    .into(),
            },
            NearToken::from_near(1),
        )
        .await
        .unwrap();

    assert!(receiver.view().await.unwrap().amount >= NearToken::from_near(1));
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_rotate(#[future] env: Env) {
    let [old_secret_key, new_secret_key] = [
        SecretKey::from_random(KeyType::ED25519),
        SecretKey::from_random(KeyType::ED25519),
    ];

    let [old_wallet_state, new_wallet_state] = [&old_secret_key, &new_secret_key]
        .map(|sk| State::<PublicKey>::new(Ed25519PublicKey(sk.public_key().unwrap_as_ed25519().0)));

    let [old_wallet_state_init, new_wallet_state_init] = [&old_wallet_state, &new_wallet_state]
        .map(|s| {
            StateInit::V1(StateInitV1 {
                code: env.wallet_global_id.clone(),
                data: s.as_storage(),
            })
        });

    let [old_wallet, new_wallet] = [&old_wallet_state_init, &new_wallet_state_init]
        .map(|s| env.account(s.derive_account_id()));

    let new_signed_request_body = RequestMessage {
        chain_id: "mainnet".to_string(),
        signer_id: new_wallet.id().clone(),
        nonce: 0,
        created_at: Deadline::now() - Duration::from_secs(60),
        timeout: new_wallet_state.nonces.timeout(),
        request: Request {
            ops: vec![],
            out: PromiseSingle::new(old_wallet.id())
                .function_call(
                    FunctionCallAction::new("w_execute_extension")
                        .attached_deposit(NearToken::from_yoctonear(1))
                        .min_gas(Gas::from_tgas(10))
                        .args_json(json!({
                            "request": Request {
                                ops: vec![
                                    WalletOp::SetSignatureMode { enable: false }
                                ],
                                out: Default::default()
                            },
                        })),
                )
                .into(),
        },
    };

    let old_signed_request_body = RequestMessage {
        chain_id: "mainnet".to_string(),
        signer_id: old_wallet.id().clone(),
        nonce: 0,
        created_at: Deadline::now() - Duration::from_secs(60),
        timeout: old_wallet_state.nonces.timeout(),
        request: Request {
            ops: vec![WalletOp::AddExtension {
                account_id: new_wallet.id().clone(),
            }],
            out: PromiseSingle::new(new_wallet.id())
                .state_init(new_wallet_state_init, NearToken::ZERO)
                .function_call(
                    FunctionCallAction::new("w_execute_signed")
                        .attached_deposit(NearToken::from_yoctonear(1))
                        .min_gas(Gas::from_tgas(20))
                        .args_json(json!({
                            "proof": sign_request(&new_secret_key, &new_signed_request_body),
                            "msg": new_signed_request_body,
                        })),
                )
                .into(),
        },
    };

    env.w_execute_signed(
        old_wallet.id(),
        old_wallet_state_init.clone(),
        old_signed_request_body.clone(),
        sign_request(&old_secret_key, &old_signed_request_body),
        NearToken::from_yoctonear(1),
    )
    .await
    .unwrap();

    assert!(!old_wallet.w_is_signature_allowed().await.unwrap());
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_zba(#[future] env: Env) {
    let secret_key = SecretKey::from_random(KeyType::ED25519);

    let wallet_state = State::<PublicKey>::new(Ed25519PublicKey(
        secret_key.public_key().unwrap_as_ed25519().0,
    ));

    let wallet_state_init = StateInit::V1(StateInitV1 {
        code: env.wallet_global_id.clone(),
        data: wallet_state.as_storage(),
    });

    let wallet = env.account(wallet_state_init.derive_account_id());

    ConcurrentNonces::new(rng())
        .take(
            (wallet_state.nonces.timeout().as_secs() * 2)
                .try_into()
                .unwrap(),
        )
        .map(|n| RequestMessage {
            chain_id: "mainnet".to_string(),
            signer_id: wallet.id().clone(),
            nonce: n,
            created_at: Deadline::now() - Duration::from_secs(60),
            timeout: wallet_state.nonces.timeout(),
            request: Request::default(),
        })
        .map(|msg| {
            let secret_key = &secret_key;
            let env = &env;
            let wallet = &wallet;
            let wallet_state_init = wallet_state_init.clone();
            async move {
                env.w_execute_signed(
                    wallet.id(),
                    wallet_state_init,
                    msg.clone(),
                    sign_request(secret_key, &msg),
                    NearToken::ZERO,
                )
                .await
                .map(|_| ())
            }
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<()>()
        .await
        .unwrap();

    dbg!(wallet.view().await.unwrap());
}

#[autoimpl(Deref using self.sandbox)]
struct Env {
    pub wallet_global_id: GlobalContractId,

    sandbox: Sandbox,
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

fn sign_request(secret_key: &SecretKey, body: &RequestMessage) -> String {
    let serialized = borsh::to_vec(&body).unwrap();
    let msg = [WALLET_DOMAIN, &serialized].concat();
    let hash = ::near_sdk::env::sha256_array(msg);
    sign_ed25519(secret_key, hash).to_string()
}

fn sign_ed25519(secret_key: &SecretKey, msg: impl AsRef<[u8]>) -> Ed25519Signature {
    match secret_key.sign(msg.as_ref()) {
        Signature::ED25519(signature) => Ed25519Signature(signature.to_bytes()),
        Signature::SECP256K1(_) => unimplemented!(),
    }
}
