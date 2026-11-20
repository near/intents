//! On-chain checks shared by the variants that sign the readable format.
//! Included by each variant's sandbox suite; not part of any contract WASM.

use std::time::Duration;

use defuse_wallet::{
    AccountId, Gas, NearToken, RequestMessage, SignatureSchema, Timestamp,
    offchain::OffchainMessage,
};
use defuse_wallet_sdk::WalletSigner;
use near_kit::{Near, rpc::FinalExecutionOutcome, transaction::Final};
use serde_json::json;

async fn call(
    near: &Near,
    account: &AccountId,
    method: &str,
    args: Vec<u8>,
    deposit: NearToken,
    gas: Gas,
) -> FinalExecutionOutcome {
    near.transaction(account)
        .call(method)
        .args_raw(args)
        .deposit(deposit)
        .gas(gas)
        .send()
        .wait_until::<Final>()
        .await
        .unwrap()
}

/// `msg` must be a fresh signed request. JSON fields that the contract does
/// not define are ignored and change nothing: the request executes once, and
/// its replay is rejected.
pub async fn check(near: &Near, account: &AccountId, msg: &RequestMessage, proof: &str) {
    let mut padded = serde_json::to_value(msg).unwrap();

    padded["max_spend"] = "1".into();
    padded["request"]["unused"] = true.into();

    let args = serde_json::to_vec(&json!({"msg": padded, "proof": proof, "unused": true})).unwrap();

    call(
        near,
        account,
        "w_execute_signed",
        args,
        NearToken::ZERO,
        Gas::from_tgas(40),
    )
    .await
    .result()
    .unwrap();

    let args = serde_json::to_vec(&json!({"msg": msg, "proof": proof})).unwrap();
    let replay = call(
        near,
        account,
        "w_execute_signed",
        args,
        NearToken::ZERO,
        Gas::from_tgas(40),
    )
    .await;

    assert!(format!("{:?}", replay.status).contains("already been used"));
}

/// Correctly signed variants of `original` for another network or account,
/// outside the validity window or paying for gas are rejected with the
/// reason, as is a NEP-641 message from the future.
pub async fn context<S: SignatureSchema>(
    near: &Near,
    account: &AccountId,
    original: &RequestMessage,
    signer: &impl WalletSigner<S>,
) {
    type Change = fn(&mut RequestMessage);
    let changes: [(&str, Change); 6] = [
        ("invalid chain_id", |msg| {
            msg.chain_id = "wrong-chain".into();
        }),
        ("invalid signer_id", |msg| {
            msg.signer_id = "wrong.near".parse().unwrap();
        }),
        ("expired or from the future", |msg| {
            msg.created_at = Timestamp::now() + Duration::from_mins(10);
        }),
        ("expired or from the future", |msg| {
            msg.created_at = Timestamp::now() - Duration::from_mins(10);
            msg.timeout = Duration::from_secs(1);
        }),
        // The wallet's own timeout (one hour) is shorter than the message's.
        ("expired or from the future", |msg| {
            msg.created_at = Timestamp::now() - Duration::from_secs(4000);
            msg.timeout = Duration::from_hours(2);
        }),
        ("`pay_for_gas` is not currently supported", |msg| {
            msg.pay_for_gas = true;
        }),
    ];

    for (index, (reason, change)) in changes.into_iter().enumerate() {
        let mut msg = original.clone();
        change(&mut msg);
        let proof = signer.sign_request_msg(&msg).await.unwrap();
        let args = serde_json::to_vec(&json!({"msg":msg,"proof":proof})).unwrap();
        let outcome = call(
            near,
            account,
            "w_execute_signed",
            args,
            NearToken::ZERO,
            Gas::from_tgas(40),
        )
        .await;
        let status = format!("{:?}", outcome.status);
        assert!(status.contains(reason), "case {index} ({reason}): {status}");
    }

    let future = OffchainMessage {
        chain_id: original.chain_id.clone(),
        signer_id: original.signer_id.clone(),
        path: Vec::new(),
        timestamp: Timestamp::now() + Duration::from_mins(10),
        payload: "from the future".to_owned(),
    };
    let proof = signer.sign_offchain_msg(&future).await.unwrap();
    let authorization = json!({"signature":{"msg":future,"proof":proof}}).to_string();
    let error = near
        .view::<serde_json::Value>(account, "w_resolve_auth")
        .args(json!({"path":[],"authorization":authorization}))
        .await
        .expect_err("NEP-641 message from the future")
        .to_string();
    assert!(error.contains("message is from the future"), "{error}");
}
