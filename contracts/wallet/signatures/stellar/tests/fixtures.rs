use std::{fmt::Write, time::Duration};

use base64::{Engine, engine::general_purpose::STANDARD};
use defuse_sep53::Sep53;
use defuse_wallet::readable_message;
use defuse_wallet::{RequestMessage, SignatureSchema, offchain::OffchainMessage};
use defuse_wallet_stellar::{
    MessageError, WalletStellar,
    crypto::ed25519::{
        Ed25519PublicKey,
        ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey},
    },
    offchain_signing_message, request_signing_message, stellar_address,
};
use serde_json::{Map, Value, json};

fn vectors() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!("fixtures/stellar.json")).unwrap()["vectors"]
        .as_array()
        .unwrap()
        .clone()
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/stellar.json")).unwrap()
}
fn key(v: &Value) -> Ed25519PublicKey {
    let address = v["address"].as_str().unwrap();
    Ed25519PublicKey(
        stellar_strkey::ed25519::PublicKey::from_string(address)
            .unwrap()
            .0,
    )
}
/// SEP-53 proof of `message` as a Stellar wallet returns it.
fn sign(signing: &SigningKey, message: &str) -> String {
    STANDARD.encode(signing.sign(&Sep53::prehash(message)).to_bytes())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut hex, byte| {
        write!(hex, "{byte:02x}").unwrap();
        hex
    })
}

#[test]
fn independent_stellar_sdk_vectors() {
    let all = vectors();
    assert_eq!(all.len(), 10);
    for v in all {
        let public_key = key(&v);
        assert_eq!(stellar_address(&public_key), v["address"]);
        assert_eq!(
            hex(&Sep53::prehash(v["message"].as_str().unwrap())),
            v["sep53_hash"]
        );
        let proof = v["proof"].as_str().unwrap();
        if v["kind"] == "request" {
            let msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            assert_eq!(
                request_signing_message(&public_key, &msg).unwrap(),
                v["message"]
            );
            assert!(WalletStellar::verify_request_msg(&public_key, &msg, proof));
        } else {
            let msg: OffchainMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            assert_eq!(
                offchain_signing_message(&public_key, &msg).unwrap(),
                v["message"]
            );
            assert!(WalletStellar::verify_offchain_msg(&public_key, &msg, proof));
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
        ("/request/internal/2/payload/decimals", json!(17)),
        ("/request/internal/2/payload/symbol", json!("TKM")),
        (
            "/request/external/0/actions/0/payload/function_name",
            json!("other"),
        ),
        (
            "/request/external/0/actions/0/payload/args",
            json!("eyJyZWNlaXZlcl9pZCI6Im90aGVyLm5lYXIiLCJhbW91bnQiOiIyIn0="),
        ),
        ("/request/external/0/actions/0/payload/deposit", json!("2")),
        ("/request/external/0/actions/0/payload/gas", json!("1")),
        (
            "/request/external/0/actions/0/payload/gas_weight",
            json!("1"),
        ),
    ];
    for (pointer, value) in mutations {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        let msg: RequestMessage = serde_json::from_value(changed).unwrap();
        assert_ne!(
            request_signing_message(&key(v), &msg).expect("mutation must render"),
            v["message"]
        );
        assert!(
            !WalletStellar::verify_request_msg(&key(v), &msg, v["proof"].as_str().unwrap()),
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
        assert!(!WalletStellar::verify_request_msg(
            &key(v),
            &msg,
            v["proof"].as_str().unwrap()
        ));
    }
    // Move the metadata requirement and receiver together; the original proof must fail.
    let mut moved: RequestMessage = serde_json::from_value(original.clone()).unwrap();
    let defuse_wallet::WalletOp::RequireFtMetadata { token_id, .. } =
        &mut moved.request.internal[2]
    else {
        unreachable!()
    };
    *token_id = "other.near".parse().unwrap();
    moved.request.external[0].receiver_id = token_id.clone();
    request_signing_message(&key(v), &moved).unwrap();
    assert!(!WalletStellar::verify_request_msg(
        &key(v),
        &moved,
        v["proof"].as_str().unwrap()
    ));
    // A native transfer has no token-metadata coupling.
    let transfer = &all[1];
    let mut msg: RequestMessage = serde_json::from_value(transfer["msg"].clone()).unwrap();
    msg.request.external[0].receiver_id = "other.near".parse().unwrap();
    request_signing_message(&key(transfer), &msg).unwrap();
    assert!(!WalletStellar::verify_request_msg(
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
            !WalletStellar::verify_request_msg(&key(v), &msg, v["proof"].as_str().unwrap()),
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
            !WalletStellar::verify_offchain_msg(&key(v), &msg, v["proof"].as_str().unwrap()),
            "accepted {pointer}"
        );
    }
}

#[test]
fn malformed_wrong_key_and_cross_domain_proofs_fail() {
    let all = vectors();
    let v = &all[0];
    let msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
    let valid = v["proof"].as_str().unwrap();
    assert!(valid.ends_with("Cw=="));
    let signature = STANDARD.decode(valid).unwrap();
    for proof in [
        String::new(),
        "not a signature".to_owned(),
        "A".repeat(88),
        // Other encodings of the valid signature.
        valid.trim_end_matches('=').to_owned(),
        // Nonzero unused bits in the last character.
        valid.replace("Cw==", "Cx=="),
        base64::engine::general_purpose::URL_SAFE.encode(&signature),
        format!(" {}", &valid[1..]),
        defuse_wallet_stellar::crypto::ed25519::Ed25519Signature(
            signature.clone().try_into().unwrap(),
        )
        .to_string(),
        hex(&signature),
        // A valid proof of another message.
        all[4]["proof"].as_str().unwrap().to_owned(),
    ] {
        assert_ne!(proof, valid);
        assert!(
            !WalletStellar::verify_request_msg(&key(v), &msg, &proof),
            "accepted {proof}"
        );
    }
    assert!(WalletStellar::verify_request_msg(&key(v), &msg, valid));
    assert!(!WalletStellar::verify_request_msg(
        &Ed25519PublicKey([0; 32]),
        &msg,
        valid
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
            .verify(&Sep53::prehash(&message), &Signature::from_bytes(&forged))
            .is_ok()
    );
    assert!(!WalletStellar::verify_request_msg(
        &Ed25519PublicKey(identity),
        &msg,
        &STANDARD.encode(forged)
    ));
    let auth: OffchainMessage = serde_json::from_value(all[4]["msg"].clone()).unwrap();
    assert!(!WalletStellar::verify_offchain_msg(&key(v), &auth, valid));
}

#[test]
fn limits_and_ambiguous_timeout_are_rejected() {
    let all = vectors();
    let v = &all[0];
    let proof = v["proof"].as_str().unwrap();
    let mut msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
    msg.timeout = Duration::from_nanos(1);
    assert!(request_signing_message(&key(v), &msg).is_err());
    assert!(!WalletStellar::verify_request_msg(&key(v), &msg, proof));
    msg.timeout = Duration::from_secs(u64::from(u32::MAX) + 1);
    assert!(request_signing_message(&key(v), &msg).is_err());
    assert!(!WalletStellar::verify_request_msg(&key(v), &msg, proof));

    // The limit applies to the signed document, which is longer than the
    // readable text of the same message.
    let signing = SigningKey::from_bytes(&[42; 32]);
    let pk = Ed25519PublicKey(signing.verifying_key().to_bytes());
    let mut auth: OffchainMessage = serde_json::from_value(all[4]["msg"].clone()).unwrap();
    let empty = offchain_signing_message(&pk, &auth).unwrap().len();
    auth.payload = "x".repeat(16 * 1024 - empty);
    let largest = offchain_signing_message(&pk, &auth).unwrap();
    assert_eq!(largest.len(), 16 * 1024);
    assert!(WalletStellar::verify_offchain_msg(
        &pk,
        &auth,
        &sign(&signing, &largest)
    ));
    auth.payload.push('x');
    let identity = stellar_address(&pk);
    readable_message::offchain_signing_message(&[("stellar_address", &identity)], &auth)
        .expect("the readable text still fits");
    assert!(matches!(
        offchain_signing_message(&pk, &auth),
        Err(MessageError::TooLarge)
    ));
    auth.payload = "x".repeat(16 * 1024);
    assert!(matches!(
        offchain_signing_message(&pk, &auth),
        Err(MessageError::Readable(
            readable_message::MessageError::TooLarge
        ))
    ));
}

/// The signed document carries exactly the readable v1 text: its prefix and
/// warning lines, then its JSON fields.
#[test]
fn document_is_the_readable_text() {
    for v in vectors() {
        let address = v["address"].as_str().unwrap();
        let identity = [("stellar_address", address)];
        let text = if v["kind"] == "request" {
            let msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            readable_message::request_signing_message(&identity, &msg)
        } else {
            let msg: OffchainMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            readable_message::offchain_signing_message(&identity, &msg)
        }
        .unwrap();
        let signed = v["message"].as_str().unwrap();
        let mut document: Map<String, Value> = serde_json::from_str(signed).unwrap();
        // A wallet that parses and prints the document shows the signed text.
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&document).unwrap()),
            signed
        );
        let mut lines = |name: &str| {
            let Some(Value::Array(lines)) = document.shift_remove(name) else {
                panic!("missing {name}");
            };
            lines
                .iter()
                .flat_map(|line| [line.as_str().unwrap(), "\n"])
                .collect::<String>()
        };
        let about = lines("about");
        let warnings = lines("warnings");
        let separator = if warnings.is_empty() { "" } else { "\n" };
        let fields = serde_json::to_string_pretty(&document).unwrap();
        assert_eq!(format!("{about}\n{warnings}{separator}{fields}\n"), text);
    }
}

/// Messages of the readable format that a wallet would show differently than
/// they are signed are not signed by this variant.
#[test]
fn messages_shown_differently_are_rejected() {
    let fixture = fixture();
    let rejected = fixture["rejected"].as_array().unwrap();
    assert_eq!(rejected.len(), 6);
    let signing = SigningKey::from_bytes(&[42; 32]);
    let pk = Ed25519PublicKey(signing.verifying_key().to_bytes());
    let address = stellar_address(&pk);
    let identity = [("stellar_address", address.as_str())];
    for v in rejected {
        let reason = v["reason"].as_str().unwrap();
        // The proof that a signer of the readable text would produce.
        let (text, error, accepted) = if v["kind"] == "request" {
            let msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            let text = readable_message::request_signing_message(&identity, &msg).expect(reason);
            let proof = sign(&signing, &text);
            (
                text,
                request_signing_message(&pk, &msg).expect_err(reason),
                WalletStellar::verify_request_msg(&pk, &msg, &proof),
            )
        } else {
            let msg: OffchainMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            let text = readable_message::offchain_signing_message(&identity, &msg).expect(reason);
            let proof = sign(&signing, &text);
            (
                text,
                offchain_signing_message(&pk, &msg).expect_err(reason),
                WalletStellar::verify_offchain_msg(&pk, &msg, &proof),
            )
        };
        assert!(!accepted, "{reason}");
        if reason.contains("reorders") {
            assert!(matches!(error, MessageError::Display), "{reason}");
        } else {
            assert!(text.contains("\\u"), "{reason}");
            assert!(matches!(error, MessageError::Text), "{reason}");
        }
    }
}

/// The size limit bounds nesting before the JSON parser's limit of 128
/// levels does: deep arguments are never rejected for another reason.
#[test]
fn nesting_is_bounded_by_size() {
    let all = vectors();
    let v = &all[9];
    let identity = [("stellar_address", v["address"].as_str().unwrap())];
    let mut deepest = 0;
    for depth in 1..=127 {
        let mut msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
        let defuse_wallet::actions::NearAction::FunctionCall(call) =
            &mut msg.request.external[0].actions[0]
        else {
            unreachable!()
        };
        call.args = format!("{}{}", "[".repeat(depth), "]".repeat(depth)).into_bytes();
        let text = readable_message::request_signing_message(&identity, &msg);
        match request_signing_message(&key(v), &msg) {
            Ok(_) => {
                assert!(text.is_ok());
                deepest = depth;
            }
            Err(MessageError::TooLarge) => assert!(text.is_ok()),
            Err(MessageError::Readable(_)) => assert!(text.is_err()),
            Err(error) => panic!("depth {depth}: {error}"),
        }
    }
    assert_eq!(deepest, 82);
}

#[test]
fn state_init_fixture_uses_derived_receiver() {
    for v in vectors() {
        if v["kind"] != "request" {
            continue;
        }
        let msg: defuse_wallet::RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
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
    let signing = SigningKey::from_bytes(&[42; 32]);
    let pk = Ed25519PublicKey(signing.verifying_key().to_bytes());
    let all = vectors();
    let msg: RequestMessage = serde_json::from_value(all[0]["msg"].clone()).unwrap();
    let message = request_signing_message(&pk, &msg).unwrap();
    // Signatures over the request digest, over the message without the SEP-53
    // hash (as Solana wallets sign) and over the readable text are not proofs.
    let identity = stellar_address(&pk);
    let text =
        readable_message::request_signing_message(&[("stellar_address", &identity)], &msg).unwrap();
    for signature in [
        signing.sign(&msg.hash()),
        signing.sign(message.as_bytes()),
        signing.sign(&Sep53::prehash(&text)),
        signing.sign(&Sep53::prehash(message.trim_end())),
    ] {
        assert!(!WalletStellar::verify_request_msg(
            &pk,
            &msg,
            &STANDARD.encode(signature.to_bytes())
        ));
    }
    let proof = sign(&signing, &message);
    assert!(
        signing
            .verifying_key()
            .verify_strict(
                &msg.hash(),
                &Signature::from_slice(&STANDARD.decode(&proof).unwrap()).unwrap()
            )
            .is_err()
    );
    assert!(WalletStellar::verify_request_msg(&pk, &msg, &proof));
    // A message that cannot be rendered does not fall back to its digest.
    let mut unrenderable = msg;
    unrenderable.timeout = Duration::from_nanos(1);
    assert!(request_signing_message(&pk, &unrenderable).is_err());
    for prehash in [unrenderable.hash(), Sep53::prehash(unrenderable.hash())] {
        assert!(!WalletStellar::verify_request_msg(
            &pk,
            &unrenderable,
            &STANDARD.encode(signing.sign(&prehash).to_bytes())
        ));
    }
}

#[tokio::test]
async fn metadata_token_id_is_bound_without_a_transfer() {
    use defuse_wallet_sdk::WalletSigner;
    use defuse_wallet_stellar::WalletStellarSigner;
    let signer = WalletStellarSigner(SigningKey::from_bytes(&[42; 32]));
    let mut msg: RequestMessage = serde_json::from_value(vectors()[2]["msg"].clone()).unwrap();
    msg.request.external.clear();
    let proof = signer.sign_request_msg(&msg).await.unwrap();
    assert!(WalletStellar::verify_request_msg(
        &signer.public_key(),
        &msg,
        &proof
    ));
    let defuse_wallet::WalletOp::RequireFtMetadata { token_id, .. } = &mut msg.request.internal[2]
    else {
        unreachable!()
    };
    *token_id = "other.near".parse().unwrap();
    request_signing_message(&signer.public_key(), &msg).unwrap();
    assert!(!WalletStellar::verify_request_msg(
        &signer.public_key(),
        &msg,
        &proof
    ));
}

/// The SDK signer returns what an independent SEP-53 implementation signs.
#[tokio::test]
async fn sdk_signer_matches_independent_vectors() {
    use defuse_wallet_sdk::WalletSigner;
    use defuse_wallet_stellar::WalletStellarSigner;
    // RFC 8032 test 1, the key of the fixture.
    let seed: [u8; 32] = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ];
    let signer = WalletStellarSigner(SigningKey::from_bytes(&seed));
    for v in vectors() {
        assert_eq!(signer.public_key(), key(&v));
        let proof = if v["kind"] == "request" {
            let msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            signer.sign_request_msg(&msg).await.unwrap()
        } else {
            let msg: OffchainMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            signer.sign_offchain_msg(&msg).await.unwrap()
        };
        assert_eq!(proof, v["proof"]);
    }
}

/// Test case 1 of SEP-53 and the address of its seed.
#[test]
fn sep53_specification_vector() {
    let address = "GBXFXNDLV4LSWA4VB7YIL5GBD7BVNR22SGBTDKMO2SBZZHDXSKZYCP7L";
    let seed = stellar_strkey::ed25519::PrivateKey::from_string(
        "SAKICEVQLYWGSOJS4WW7HZJWAHZVEEBS527LHK5V4MLJALYKICQCJXMW",
    )
    .unwrap();
    let signing = SigningKey::from_bytes(&seed.0);
    assert_eq!(
        stellar_address(&Ed25519PublicKey(signing.verifying_key().to_bytes())),
        address
    );
    assert_eq!(
        sign(&signing, "Hello, World!"),
        "fO5dbYhXUhBMhe6kId/cuVq/AfEnHRHEvsP8vXh03M1uLpi5e46yO2Q8rEBzu3feXQewcQE5GArp88u6ePK6BA=="
    );
}

/// Frozen vectors computed outside this repository: `0s` and the last 20
/// bytes of `keccak256(borsh(StateInit))`. A change of the stored identity or
/// of the state layout changes every wallet address.
#[test]
fn derived_accounts_match_known_answers() {
    use defuse_wallet::{GlobalContractId, State, StateInit, StateInitV1};
    let all = vectors();
    assert_eq!(
        key(&all[0]).to_string(),
        "ed25519:FVen3X669xLzsi6N2V91DoiyzHzg1uAgqiT8jZ9nS96Z"
    );
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
