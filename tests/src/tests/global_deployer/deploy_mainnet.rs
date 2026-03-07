#![cfg(feature = "wallet")]

use std::{sync::Arc, time::Duration};

use defuse_global_deployer::State as DeployerState;
use defuse_sandbox::api::{
    self, Account, Contract, NetworkConfig, Signer,
    signer::generate_secret_key,
    types::{
        Action,
        transaction::actions::{
            DeterministicAccountStateInit, DeterministicAccountStateInitV1,
            DeterministicStateInitAction, FunctionCallAction, GlobalContractIdentifier,
        },
    },
};
use defuse_test_utils::wasms::{WALLET_ED25519_WASM, WALLET_WEBAUTHN_ED25519_WASM};
use defuse_wallet::{
    AddExtensionOp, PromiseDAG, PromiseSingle, RemoveExtensionOp, Request, State as WalletState,
    WalletOp,
    signature::{
        Deadline, RequestMessage,
        ed25519::{Ed25519PublicKey, Ed25519Signature},
        webauthn::{ClientDataType, CollectedClientData, Ed25519, PayloadSignature},
    },
};
use hex_literal::hex;
use near_crypto::{KeyType, SecretKey, Signature};
use near_sdk::{
    AccountId, AccountIdRef, Gas, GlobalContractId, NearToken, borsh, env,
    state_init::{StateInit, StateInitV1},
};
use serde_json::json;

const GD_MUTABLE_ID: &AccountIdRef =
    AccountIdRef::new_or_panic("0s78b3344b3bf415446e7b48c5a67955fd3f5cbf30");
const RELAYER_ID: &AccountIdRef = AccountIdRef::new_or_panic("defuse-ops.near");
const INDEX: u32 = 2;

const WALLET_ED25519_HASH: [u8; 32] =
    hex!("f1bdf3f2ea553f39ddd2d0474a1fa04c5aca2956bf4dcddb0370ea7fdd5ae5dc");

const WALLET_GLOBAL_ID: &AccountIdRef =
    AccountIdRef::new_or_panic("0s02f3ec89711e53d27a7a569444713189f57d9881");

const WALLET_WEBAUTHN_ED25519_GLOBAL_ID: &AccountIdRef =
    AccountIdRef::new_or_panic("0seeb983e7654fec5189212902fa54de49e9223eb2");

#[tokio::test]
async fn deploy_wallet_to_mainnet() {
    assert_eq!(
        env::sha256_array(&*WALLET_ED25519_WASM),
        WALLET_ED25519_HASH
    );
    gd_deploy(GD_MUTABLE_ID.to_owned(), &*WALLET_WEBAUTHN_ED25519_WASM).await;
}

// #[tokio::test]
// async fn deploy_use_wallet() {
//     let secret_key = SecretKey::from_random(KeyType::ED25519);

//     w_execute_signed(
//         &secret_key,
//         Request {
//             ops: vec![],
//             out: PromiseSingle::new("v1.signer".parse::<AccountId>().unwrap())
//                 .function_call(
//                     defuse_wallet::FunctionCallAction::new("sign")
//                         .args_json(json!({
//                             "request": {
//                                 "payload_v2": {
//                                     "Eddsa": "eca9db47e63166d5896f69f68d02bf9de314673e56ee2e4a2ccd5dba8364d511"
//                                 },
//                                 "path": "",
//                                 "domain_id": 1,
//                             },
//                         }))
//                         .attached_deposit(NearToken::from_yoctonear(1)),
//                 )
//                 .into(),
//         },
//     )
//     .await;
// }

async fn w_execute_signed(secret_key: &SecretKey, request: Request) {
    println!("secret_key: {secret_key}");
    let public_key = secret_key.public_key();
    println!("public_key: {public_key}");

    let wallet_state_init = StateInit::V1(StateInitV1 {
        code: WALLET_WEBAUTHN_ED25519_GLOBAL_ID.to_owned().into(),
        data: WalletState::new(public_key.unwrap_as_ed25519().0).init_state(),
    });
    let wallet_id = wallet_state_init.derive_account_id();
    println!("wallet_id: {wallet_id}");

    let seqno = Contract(wallet_id.clone())
        .call_function("w_seqno", ())
        .read_only::<u32>()
        .fetch_from_mainnet()
        .await
        .map(|r| r.data)
        .unwrap_or(0);

    let signed = RequestMessage {
        chain_id: "mainnet".to_string(),
        signer_id: wallet_id.clone(),
        seqno,
        valid_until: Deadline::timeout(Duration::from_secs(60)),
        request,
    };

    let proof = sign_request(secret_key, &signed).to_string();

    let tx = api::Transaction::construct(RELAYER_ID.to_owned(), wallet_id)
        .add_action(action_state_init(wallet_state_init))
        .add_action(action_w_execute_signed(signed, proof));

    println!(
        "signing tx:\n{} -> {} ({} actions)",
        tx.transaction.clone().unwrap().signer_id,
        tx.transaction.clone().unwrap().receiver_id,
        tx.transaction.clone().unwrap().actions.len()
    );

    let signer = Signer::from_keystore_with_search_for_keys(
        RELAYER_ID.to_owned(),
        &NetworkConfig::mainnet(),
    )
    .await
    .unwrap();

    tx.with_signer(signer)
        .send_to_mainnet()
        .await
        .unwrap()
        .into_result()
        .unwrap();
}

async fn gd_deploy(global_contract_id: impl Into<GlobalContractId>, code: impl AsRef<[u8]>) {
    let gd_state_init = StateInit::V1(StateInitV1 {
        code: global_contract_id.into(),
        data: DeployerState::new(RELAYER_ID, INDEX).state_init(),
    });

    let gd_account_id = gd_state_init.derive_account_id();
    println!("gd_account_id: {gd_account_id}");

    let tx = api::Transaction::construct(RELAYER_ID.to_owned(), gd_account_id)
        .add_action(action_state_init(gd_state_init))
        .add_action(action_gd_deploy(DeployerState::DEFAULT_HASH, code));

    println!(
        "signing tx:\n{} -> {} ({} actions)",
        tx.transaction.clone().unwrap().signer_id,
        tx.transaction.clone().unwrap().receiver_id,
        tx.transaction.clone().unwrap().actions.len()
    );

    let signer = Signer::from_keystore_with_search_for_keys(
        RELAYER_ID.to_owned(),
        &NetworkConfig::mainnet(),
    )
    .await
    .unwrap();

    tx.with_signer(signer)
        .send_to_mainnet()
        .await
        .unwrap()
        .into_result()
        .unwrap();
}

fn action_state_init(state_init: StateInit) -> Action {
    Action::DeterministicStateInit(
        DeterministicStateInitAction {
            state_init: state_init_sdk2api(state_init),
            deposit: NearToken::ZERO,
        }
        .into(),
    )
}

fn action_w_execute_signed(signed: RequestMessage, proof: String) -> Action {
    let args = serde_json::to_string_pretty(&json!({
        "signed": signed,
        "proof": proof,
    }))
    .unwrap();

    println!("{}::w_execute_signed({args})", &signed.signer_id);

    Action::FunctionCall(
        FunctionCallAction {
            method_name: "w_execute_signed".to_string(),
            args: args.into_bytes(),
            gas: Gas::from_tgas(250),
            deposit: NearToken::from_yoctonear(1),
        }
        .into(),
    )
}

fn sign_request(secret_key: &SecretKey, body: &RequestMessage) -> String {
    let domain = body.wrap_domain();
    let serialized = borsh::to_vec(&domain).unwrap();
    serde_json::to_string(&sign_passkey(secret_key, &env::sha256_array(serialized))).unwrap()
    // sign_ed25519(secret_key, serialized).to_string()
}

fn sign_passkey(secret_key: &SecretKey, msg: &[u8]) -> PayloadSignature<Ed25519> {
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

    let hash = env::sha256_array(client_data_json.as_bytes());

    PayloadSignature {
        signature: sign_ed25519(
            secret_key,
            &[authenticator_data.as_slice(), hash.as_slice()].concat(),
        ),
        authenticator_data,
        client_data_json,
    }
}

fn sign_ed25519(secret_key: &SecretKey, msg: impl AsRef<[u8]>) -> Ed25519Signature {
    match secret_key.sign(msg.as_ref()) {
        Signature::ED25519(signature) => Ed25519Signature(signature.to_bytes()),
        Signature::SECP256K1(_) => unimplemented!(),
    }
}

fn action_gd_deploy(old_hash: [u8; 32], new_code: impl AsRef<[u8]>) -> Action {
    Action::FunctionCall(
        FunctionCallAction {
            method_name: "gd_deploy".to_string(),
            args: gd_deploy_args(old_hash, new_code.as_ref()),
            gas: Gas::from_tgas(200),
            deposit: NearToken::from_near(50),
        }
        .into(),
    )
}

fn gd_deploy_args(old_hash: [u8; 32], new_code: &[u8]) -> Vec<u8> {
    borsh::to_vec(&(old_hash, new_code)).unwrap()
}

fn state_init_sdk2api(state_init: StateInit) -> DeterministicAccountStateInit {
    match state_init {
        StateInit::V1(StateInitV1 { code, data }) => {
            DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
                code: match code {
                    GlobalContractId::CodeHash(hash) => {
                        GlobalContractIdentifier::CodeHash(near_api::CryptoHash(*hash.as_ref()))
                    }
                    GlobalContractId::AccountId(account) => {
                        GlobalContractIdentifier::AccountId(account)
                    }
                },
                data,
            })
        }
    }
}
