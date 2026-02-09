use std::time::Duration;

use defuse_crypto::{Ed25519PublicKey, Ed25519Signature};
use defuse_deadline::Deadline;
use defuse_tests::{
    crypto::{KeyType, SecretKey},
    env::WALLET_WEBAUTHN_ED25519_WASM,
    sandbox::{
        FnCallBuilder, Sandbox,
        api::{
            CryptoHash,
            types::transaction::actions::{
                DeterministicAccountStateInit, DeterministicAccountStateInitV1,
                GlobalContractIdentifier,
            },
        },
        sandbox,
    },
};
use defuse_wallet::{
    self, PromiseDAG, PromiseSingle, Request, SignedRequest, State, webauthn::Webauthn,
};
use defuse_webauthn::{ClientDataType, CollectedClientData, Ed25519, PayloadSignature};
use impl_tools::autoimpl;
use near_sdk::{
    GlobalContractId, NearToken, borsh,
    env::sha256_array,
    serde_json::{self, json},
    state_init::{StateInit, StateInitV1},
};
use rstest::{fixture, rstest};

#[rstest]
#[awt]
#[tokio::test]
async fn test_signed(#[future] env: Env) {
    let secret_key = SecretKey::from_random(KeyType::ED25519);

    let wallet_state_init = StateInit::V1(StateInitV1 {
        code: env.wallet_global_id.clone(),
        data: State::<Webauthn<Ed25519>>::new(Ed25519PublicKey(
            secret_key.public_key().unwrap_as_ed25519().0,
        ))
        .init_state(),
    });

    let wallet = env.account(wallet_state_init.derive_account_id());

    let request = Request {
        ops: vec![],
        out: PromiseDAG::default(),
        // out: PromiseDAG::new(
        //     PromiseSingle::new(wallet.id())
        //         .state_init(wallet_state_init.clone(), NearToken::ZERO)
        //         .transfer(NearToken::from_near(1)),
        // ),
    };

    let signed_request_body = SignedRequest {
        signer_id: wallet.id().clone(),
        chain_id: "mainnet".to_string(),
        valid_until: Deadline::timeout(Duration::from_secs(60 * 60)), // 1h
        seqno: 0,
        request,
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
                    "proof": serde_json::to_string(&sign_request(secret_key, &signed_request_body)).unwrap(),
                    "signed": signed_request_body,
                }))
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

    let other = env
        .generate_subaccount("other", NearToken::ZERO)
        .await
        .unwrap();

    let refund_to = env
        .generate_subaccount("refund_to", NearToken::ZERO)
        .await
        .unwrap();

    let wallet_state_init = StateInit::V1(StateInitV1 {
        code: env.wallet_global_id.clone(),
        data: State::<Webauthn<Ed25519>>::new(Ed25519PublicKey([0; 32]))
            .extensions([extension.id()])
            .init_state(),
    });

    // 0s123445
    let wallet = env.account(wallet_state_init.derive_account_id());

    extension
        .tx(wallet.id())
        .state_init(wallet_state_init.clone(), NearToken::ZERO)
        .function_call(
            FnCallBuilder::new("w_execute_extension")
                .json_args(json!({
                    "request": Request {
                        ops: vec![],
                        out: PromiseDAG::new(
                            PromiseSingle::new(wallet.id())
                                .state_init(wallet_state_init, NearToken::ZERO)
                                .transfer(NearToken::from_near(1))
                        ),
                    }
                }))
                .with_deposit(NearToken::from_near(1)),
        )
        .await
        .unwrap();

    assert!(wallet.view().await.unwrap().amount >= NearToken::from_near(1));
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
            WALLET_WEBAUTHN_ED25519_WASM.clone(),
        )
        .await
        .unwrap();

    Env {
        wallet_global_id: wallet_contract.id().clone().into(),
        sandbox,
    }
}

fn sign_request(secret_key: SecretKey, body: &SignedRequest) -> PayloadSignature<Ed25519> {
    sign_passkey(secret_key, &body.hash())
}

fn sign_passkey(secret_key: SecretKey, msg: &[u8]) -> PayloadSignature<Ed25519> {
    let authenticator_data = {
        let mut buf = [0; 37];
        buf[32] = 0b0000_0001;
        buf.to_vec()
    };

    let c = CollectedClientData {
        typ: ClientDataType::Get,
        challenge: msg.to_vec(),
        origin: "example.com".to_string(),
    };

    let client_data_json = serde_json::to_string(&c).unwrap();

    let hash = sha256_array(client_data_json.as_bytes());

    let signature =
        match secret_key.sign(&[authenticator_data.as_slice(), hash.as_slice()].concat()) {
            defuse_tests::crypto::Signature::ED25519(signature) => signature.to_bytes(),
            defuse_tests::crypto::Signature::SECP256K1(_) => unimplemented!(),
        };

    PayloadSignature {
        authenticator_data,
        client_data_json,
        signature: Ed25519Signature(signature),
    }
}
