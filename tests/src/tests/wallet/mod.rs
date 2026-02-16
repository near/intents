use std::time::Duration;

use defuse_sandbox::{FnCallBuilder, Sandbox, sandbox};
use defuse_test_utils::{random::make_arbitrary, wasms::WALLET_ED25519_WASM};
use defuse_wallet::{
    self, PromiseSingle, Request, State, WalletOp,
    signature::{
        Borsh, Deadline, RequestMessage, SigningStandard,
        ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
    },
};
use impl_tools::autoimpl;
use near_crypto::{KeyType, SecretKey, Signature};
use near_sdk::{
    GlobalContractId, NearToken, borsh,
    serde_json::{self, json},
    state_init::{StateInit, StateInitV1},
};
use rstest::{fixture, rstest};

type S = Borsh<Ed25519>;
type PublicKey = <S as SigningStandard<RequestMessage>>::PublicKey;

#[rstest]
#[awt]
#[tokio::test]
async fn test_signed(#[future] env: Env) {
    let secret_key = SecretKey::from_random(KeyType::ED25519);

    let wallet_state_init = StateInit::V1(StateInitV1 {
        code: env.wallet_global_id.clone(),
        data: State::<PublicKey>::new(Ed25519PublicKey(
            secret_key.public_key().unwrap_as_ed25519().0,
        ))
        .init_state(),
    });

    let wallet = env.account(wallet_state_init.derive_account_id());

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
        signer_id: wallet.id().clone(),
        chain_id: "mainnet".to_string(),
        valid_until: Deadline::timeout(Duration::from_secs(60 * 60)), // 1h
        seqno: 0,
        request,
    };

    env.tx(wallet.id())
        .state_init(wallet_state_init.clone(), NearToken::ZERO)
        .transfer(NearToken::from_near(1))
        .function_call(
            FnCallBuilder::new("w_execute_signed")
                .json_args(dbg!(json!({
                    "proof": sign_request(&secret_key, &signed_request_body),
                    "msg": signed_request_body,
                })))
                .with_deposit(NearToken::from_near(1)),
        )
        .await
        .unwrap();

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
            .init_state(),
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
        .tx(wallet.id())
        .state_init(wallet_state_init.clone(), NearToken::ZERO)
        .function_call(
            FnCallBuilder::new("w_execute_extension")
                .json_args(json!({
                    "request": Request {
                        ops: vec![WalletOp::RemoveExtension{
                            account_id: extension.id().clone()
                        }],
                        out: PromiseSingle::new(receiver.id())
                                .refund_to(refund_to.id())
                                .transfer(NearToken::from_near(1))
                                .into(),
                    }
                }))
                .with_deposit(NearToken::from_near(1)),
        )
        .await
        .unwrap();

    assert!(receiver.view().await.unwrap().amount >= NearToken::from_near(1));
}

#[ignore]
#[rstest]
#[awt]
#[tokio::test]
async fn test_arbitrary(#[future] env: Env, #[from(make_arbitrary)] request: Request) {
    let secret_key = SecretKey::from_random(KeyType::ED25519);

    let wallet_state_init = StateInit::V1(StateInitV1 {
        code: env.wallet_global_id.clone(),
        data: State::<PublicKey>::new(Ed25519PublicKey(
            secret_key.public_key().unwrap_as_ed25519().0,
        ))
        .init_state(),
    });

    let wallet = env.account(wallet_state_init.derive_account_id());

    // let receiver = env
    //     .generate_subaccount("receiver", NearToken::ZERO)
    //     .await
    //     .unwrap();

    let signed_request_body = RequestMessage {
        signer_id: wallet.id().clone(),
        chain_id: "mainnet".to_string(),
        valid_until: Deadline::timeout(Duration::from_secs(60 * 60)), // 1h
        seqno: 0,
        request: dbg!(request),
    };

    env.tx(wallet.id())
        .state_init(
            wallet_state_init.clone(),
            NearToken::ZERO,
        )
        .transfer(NearToken::from_near(1))
        .function_call(
            FnCallBuilder::new("w_execute_signed")
                .json_args(json!({
                    "proof": serde_json::to_string(&sign_request(&secret_key, &signed_request_body)).unwrap(),
                    "signed": signed_request_body,
                }))
                .with_deposit(NearToken::from_near(1)),
        )
        .await
        .unwrap();
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
        .deploy_global_sub_contract(
            "wallet",
            NearToken::from_near(1000),
            WALLET_ED25519_WASM.clone(),
        )
        .await
        .unwrap();

    Env {
        wallet_global_id: wallet_contract.id().clone().into(),
        sandbox,
    }
}

fn sign_request(secret_key: &SecretKey, body: &RequestMessage) -> String {
    let domain = body.with_domain();
    let serialized = borsh::to_vec(&domain).unwrap();
    // let hash = near_sdk::env::sha256_array(serialized);
    // sign_passkey(secret_key, &hash)
    sign_ed25519(secret_key, serialized).to_string()
}

// fn sign_passkey(secret_key: &SecretKey, msg: &[u8]) -> PayloadSignature<Ed25519> {
//     let authenticator_data = {
//         let mut buf = [0; 37];
//         buf[32] = 0b0000_0001;
//         buf.to_vec()
//     };

//     let c = CollectedClientData {
//         typ: ClientDataType::Get,
//         challenge: msg.to_vec(),
//         origin: "example.com".to_string(),
//     };

//     let client_data_json = serde_json::to_string(&c).unwrap();

//     let hash = sha256_array(client_data_json.as_bytes());

//     PayloadSignature {
//         signature: sign_ed25519(
//             secret_key,
//             &[authenticator_data.as_slice(), hash.as_slice()].concat(),
//         ),
//         authenticator_data,
//         client_data_json,
//     }
// }

fn sign_ed25519(secret_key: &SecretKey, msg: impl AsRef<[u8]>) -> Ed25519Signature {
    match secret_key.sign(msg.as_ref()) {
        Signature::ED25519(signature) => Ed25519Signature(signature.to_bytes()),
        Signature::SECP256K1(_) => unimplemented!(),
    }
}
