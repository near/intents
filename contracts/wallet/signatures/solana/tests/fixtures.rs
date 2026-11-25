use std::{fmt::Write, time::Duration};

use defuse_wallet::{
    AccountId, Gas, NearPromise, Request, RequestMessage, SignatureSchema, actions::FunctionCall,
    offchain::OffchainMessage,
};
use defuse_wallet_solana::{
    MessageError, WalletSolana,
    crypto::ed25519::{
        Ed25519PublicKey, Ed25519Signature,
        ed25519_dalek::{Signature, Verifier, VerifyingKey},
    },
    offchain_signing_message, request_signing_message,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn vectors() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!("fixtures/solana.json")).unwrap()["vectors"]
        .as_array()
        .unwrap()
        .clone()
}
fn key(v: &Value) -> Ed25519PublicKey {
    format!("ed25519:{}", v["public_key"].as_str().unwrap())
        .parse()
        .unwrap()
}

#[test]
fn nested_extension_keeps_the_v1_size_boundary() {
    let vector: Value =
        serde_json::from_str(include_str!("fixtures/nested-extension-limit.json")).unwrap();
    let public_key = key(&vector);
    let padding = |name: &str| usize::try_from(vector[name].as_u64().unwrap()).unwrap();
    let promise =
        |name: &str| NearPromise::new(vector[name].as_str().unwrap().parse::<AccountId>().unwrap());
    let render = |length| {
        let inner = promise("inner_receiver").function_call(
            FunctionCall::name(vector["inner_method"].as_str().unwrap())
                .gas(Gas::from_gas(0))
                .args_json("x".repeat(length)),
        );
        let outer = promise("outer_receiver").function_call(
            FunctionCall::name("w_execute_extension")
                .gas(Gas::from_gas(0))
                .args_json(json!({ "request": Request::from(inner) })),
        );
        let mut msg: RequestMessage = serde_json::from_value(vector["msg"].clone()).unwrap();
        msg.request = outer.into();

        request_signing_message(&public_key, &msg)
    };

    let text = render(padding("accepted_padding")).unwrap();
    let mut digest = String::new();

    for byte in Sha256::digest(text.as_bytes()) {
        write!(digest, "{byte:02x}").unwrap();
    }

    assert_eq!(vector["message_bytes"], text.len());
    assert_eq!(vector["message_sha256"], digest);

    let first_rejected = padding("rejected_padding");
    assert!(render(first_rejected - 1).is_ok());
    assert!(matches!(
        render(first_rejected),
        Err(MessageError::TooLarge)
    ));
}

#[test]
fn independent_openssl_vectors() {
    for v in vectors() {
        let public_key = key(&v);
        let proof = v["proof"].as_str().unwrap();

        if v["kind"] == "request" {
            let msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            assert_eq!(
                request_signing_message(&public_key, &msg).unwrap(),
                v["message"]
            );
            assert!(WalletSolana::verify_request_msg(&public_key, &msg, proof));
        } else {
            let msg: OffchainMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            assert_eq!(
                offchain_signing_message(&public_key, &msg).unwrap(),
                v["message"]
            );
            assert!(WalletSolana::verify_offchain_msg(&public_key, &msg, proof));
        }
    }
}

#[test]
fn execution_fields_are_bound() {
    let all = vectors();
    let v = &all[2];
    let original = &v["msg"];
    let mutations = [
        ("/chain_id", json!("testnet")),
        ("/signer_id", json!("other.near")),
        ("/nonce", json!(43)),
        ("/created_at", json!("2026-11-03T12:34:56.123456788Z")),
        ("/timeout_secs", json!(301)),
        ("/pay_for_gas", json!(false)),
        ("/request/internal/0/payload/enable", json!(true)),
        (
            "/request/internal/1/payload/account_id",
            json!("other.near"),
        ),
        ("/request/internal/1/op", json!("add_extension")),
        (
            "/request/external/0/actions/0/payload/function_name",
            json!("other"),
        ),
        (
            "/request/external/0/actions/0/payload/args",
            json!("eyJyZWNlaXZlcl9pZCI6Im90aGVyLm5lYXIiLCJhbW91bnQiOiIyIn0="),
        ),
        // Mutate the generic call: a non-one-yocto FT deposit is invalid before verification.
        ("/request/external/0/actions/1/payload/deposit", json!("2")),
        ("/request/external/0/actions/0/payload/gas", json!("1")),
        (
            "/request/external/0/actions/0/payload/gas_weight",
            json!("1"),
        ),
    ];

    for (pointer, value) in mutations {
        let mut changed = original.clone();
        // Optional fields may be absent in the wire input, but their defaults are signed.
        let (parent, field) = pointer.rsplit_once('/').unwrap();
        changed
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.to_owned(), value);
        let msg: RequestMessage = serde_json::from_value(changed).unwrap();

        assert_ne!(
            request_signing_message(&key(v), &msg).expect("mutation must render"),
            v["message"]
        );
        assert!(
            !WalletSolana::verify_request_msg(&key(v), &msg, v["proof"].as_str().unwrap()),
            "accepted {pointer}"
        );
    }

    for pointer in ["/request/internal", "/request/external/0/actions"] {
        let mut changed = original.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .reverse();
        let msg: RequestMessage = serde_json::from_value(changed).unwrap();
        assert_ne!(
            request_signing_message(&key(v), &msg).expect("mutation must render"),
            v["message"]
        );
        assert!(!WalletSolana::verify_request_msg(
            &key(v),
            &msg,
            v["proof"].as_str().unwrap()
        ));
    }
    // The token contract is part of the signature.
    let mut moved: RequestMessage = serde_json::from_value(original.clone()).unwrap();

    moved.request.external[0].receiver_id = "other.near".parse().unwrap();
    request_signing_message(&key(v), &moved).unwrap();

    assert!(!WalletSolana::verify_request_msg(
        &key(v),
        &moved,
        v["proof"].as_str().unwrap()
    ));

    // The native transfer recipient is bound independently as well.
    let transfer = &all[1];
    let mut msg: RequestMessage = serde_json::from_value(transfer["msg"].clone()).unwrap();

    msg.request.external[0].receiver_id = "other.near".parse().unwrap();
    request_signing_message(&key(transfer), &msg).unwrap();

    assert!(!WalletSolana::verify_request_msg(
        &key(transfer),
        &msg,
        transfer["proof"].as_str().unwrap()
    ));
}

#[test]
fn transfer_refund_and_state_init_are_bound() {
    let all = vectors();

    for (index, pointer, value) in [
        (1, "/request/external/0/refund_to", json!("other.near")),
        (
            1,
            "/request/external/0/actions/0/payload/amount",
            json!("1"),
        ),
        (
            3,
            "/request/external/0/actions/0/payload/deposit",
            json!("10"),
        ),
        (
            3,
            "/request/external/0/actions/0/payload/state_init/V1/code/account_id",
            json!("other.near"),
        ),
        (
            3,
            "/request/external/0/actions/0/payload/state_init/V1/data/AAEC",
            json!("AQ=="),
        ),
    ] {
        let v = &all[index];
        let mut changed = v["msg"].clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        let msg: RequestMessage = serde_json::from_value(changed).unwrap();
        assert_ne!(
            request_signing_message(&key(v), &msg).expect("mutation must render"),
            v["message"]
        );
        assert!(
            !WalletSolana::verify_request_msg(&key(v), &msg, v["proof"].as_str().unwrap()),
            "accepted {pointer}"
        );
    }
}

#[test]
fn offchain_fields_are_bound_without_normalizing_payload() {
    let all = vectors();
    let v = &all[5];

    for (pointer, value) in [
        ("/chain_id", json!("mainnet")),
        ("/signer_id", json!("other.near")),
        ("/path", json!(["app.near", "parent.near"])),
        ("/path", json!([])),
        ("/path", json!(["parent.near", "app.near", "other.near"])),
        ("/timestamp", json!("2026-11-03T12:34:56.120000001Z")),
        ("/payload", json!("different")),
        (
            "/payload",
            json!(v["msg"]["payload"].as_str().unwrap().replace(' ', "")),
        ),
    ] {
        let mut changed = v["msg"].clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        let msg: OffchainMessage = serde_json::from_value(changed).unwrap();

        assert_ne!(
            offchain_signing_message(&key(v), &msg).expect("mutation must render"),
            v["message"]
        );
        assert!(
            !WalletSolana::verify_offchain_msg(&key(v), &msg, v["proof"].as_str().unwrap()),
            "accepted {pointer}"
        );
    }
}

#[test]
fn malformed_wrong_key_and_cross_domain_proofs_fail() {
    let all = vectors();
    let v = &all[0];
    let msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();

    for proof in [
        "",
        "ed25519:",
        "secp256k1:invalid",
        "not a signature",
        all[4]["proof"].as_str().unwrap(),
    ] {
        assert!(!WalletSolana::verify_request_msg(&key(v), &msg, proof));
    }
    assert!(!WalletSolana::verify_request_msg(
        &Ed25519PublicKey([0; 32]),
        &msg,
        v["proof"].as_str().unwrap()
    ));

    // With the identity point as the key, R = identity and s = 0 satisfy the
    // Ed25519 equation for every message (the NEAR host function accepts
    // such a forgery); only the weak-key check rejects it.
    let mut identity = [0; 32];
    identity[0] = 1;

    let mut forged = [0; 64];
    forged[0] = 1;

    let message = request_signing_message(&Ed25519PublicKey(identity), &msg).unwrap();

    assert!(
        VerifyingKey::from_bytes(&identity)
            .unwrap()
            .verify(message.as_bytes(), &Signature::from_bytes(&forged))
            .is_ok()
    );
    assert!(!WalletSolana::verify_request_msg(
        &Ed25519PublicKey(identity),
        &msg,
        &Ed25519Signature(forged).to_string()
    ));
    let auth: OffchainMessage = serde_json::from_value(all[4]["msg"].clone()).unwrap();
    assert!(!WalletSolana::verify_offchain_msg(
        &key(v),
        &auth,
        v["proof"].as_str().unwrap()
    ));
}

#[test]
fn limits_and_ambiguous_timeout_are_rejected() {
    let all = vectors();
    let v = &all[0];
    let mut msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();

    msg.timeout = Duration::from_nanos(1);

    assert!(request_signing_message(&key(v), &msg).is_err());
    assert!(!WalletSolana::verify_request_msg(
        &key(v),
        &msg,
        v["proof"].as_str().unwrap()
    ));

    msg.timeout = Duration::from_secs(u64::from(u32::MAX) + 1);

    assert!(request_signing_message(&key(v), &msg).is_err());
    assert!(!WalletSolana::verify_request_msg(
        &key(v),
        &msg,
        v["proof"].as_str().unwrap()
    ));

    let mut auth: OffchainMessage = serde_json::from_value(all[4]["msg"].clone()).unwrap();
    auth.payload = "x".repeat(16 * 1024);

    assert!(offchain_signing_message(&key(v), &auth).is_err());

    auth.payload = "bidi\u{202e} and emoji 🌍".to_string();
    let rendered = offchain_signing_message(&key(v), &auth).unwrap();

    assert!(rendered.is_ascii());
    assert!(rendered.contains("\\u202e"));
    assert!(rendered.contains("\\ud83c\\udf0d"));
}

#[test]
fn state_init_fixture_uses_derived_receiver() {
    let table: Value = serde_json::from_str(include_str!("fixtures/solana.json")).unwrap();

    for v in table["vectors"].as_array().unwrap() {
        if v["kind"] != "request" {
            continue;
        }
        let msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();

        for promise in msg.request.external {
            for action in promise.actions {
                if let defuse_wallet::actions::NearAction::DeterministicStateInit(init) = action {
                    assert_eq!(promise.receiver_id, init.state_init.derive_account_id());
                }
            }
        }
    }
}

#[test]
fn readable_and_digest_signatures_are_not_interchangeable() {
    use defuse_wallet_solana::crypto::ed25519::ed25519_dalek::{Signer, SigningKey};
    let signing = SigningKey::from_bytes(&[42; 32]);
    let pk = Ed25519PublicKey(signing.verifying_key().to_bytes());
    let all = vectors();
    let msg: RequestMessage = serde_json::from_value(all[0]["msg"].clone()).unwrap();
    let digest_proof = Ed25519Signature::from(signing.sign(&msg.hash())).to_string();
    assert!(!WalletSolana::verify_request_msg(&pk, &msg, &digest_proof));

    let readable = signing.sign(request_signing_message(&pk, &msg).unwrap().as_bytes());
    assert!(
        signing
            .verifying_key()
            .verify_strict(&msg.hash(), &readable)
            .is_err()
    );
    assert!(WalletSolana::verify_request_msg(
        &pk,
        &msg,
        &Ed25519Signature::from(readable).to_string()
    ));

    // A message that cannot be rendered does not fall back to its digest.
    let mut unrenderable = msg;
    unrenderable.timeout = Duration::from_nanos(1);
    assert!(request_signing_message(&pk, &unrenderable).is_err());

    let digest_proof = Ed25519Signature::from(signing.sign(&unrenderable.hash())).to_string();
    assert!(!WalletSolana::verify_request_msg(
        &pk,
        &unrenderable,
        &digest_proof
    ));
}

/// Frozen vectors computed outside this repository: `0s` and the last 20
/// bytes of `keccak256(borsh(StateInit))`. A change of the stored identity or
/// of the state layout changes every wallet address.
#[test]
fn derived_accounts_match_known_answers() {
    use defuse_wallet::{GlobalContractId, State, StateInit, StateInitV1};
    let all = vectors();

    for (index, subwallet_id, timeout, expected) in [
        (0, 0, 3600, "0s7956b08ff10093c0ae71a67bee136d5f95e474e0"),
        (0, 1, 3600, "0sb2bb10b40eef60b31d55e2db6e2ff2dc5fc3dc45"),
        (0, 0, 300, "0s3f32f69c330acd19eb73d4d2174fcfecbe26aa48"),
    ] {
        let state = State::new(key(&all[index]))
            .subwallet_id(subwallet_id)
            .timeout(Duration::from_secs(timeout));
        let account = StateInit::V1(StateInitV1 {
            code: GlobalContractId::CodeHash([0x11; 32]),
            data: state.as_storage(),
        })
        .derive_account_id();
        assert_eq!(account.as_str(), expected);
    }
}
