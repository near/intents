//! Shared on-chain regressions for the public entrypoints of signing variants.
//! Included by each variant's sandbox suite; not part of any contract WASM.

use std::time::Duration;

use defuse_wallet::{
    AccountId, Gas, NearPromise, NearToken, Request, RequestMessage, SignatureSchema, Timestamp,
    WalletOp, actions::FunctionCall, offchain::OffchainMessage,
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
        .finish()
        .wait_until::<Final>()
        .await
        .unwrap()
}

/// `msg` must be a fresh request enabling the sandbox root as an extension.
/// Failed argument parsing must not consume it; the clean request must execute.
pub async fn check(near: &Near, account: &AccountId, msg: &RequestMessage, proof: &str) {
    let private = call(
        near,
        account,
        "w_on_ft_metadata",
        Vec::new(),
        NearToken::ZERO,
        Gas::from_tgas(30),
    )
    .await;
    let status = format!("{:?}", private.status);
    assert!(status.contains("private"), "{status}");

    let serialized = serde_json::to_string(msg).unwrap();
    for extra in [
        r#","unused":true"#.to_owned(),
        format!(",\"proof\":\"{proof}\""),
    ] {
        let args = format!("{{\"msg\":{serialized},\"proof\":\"{proof}\"{extra}}}");
        let result = call(
            near,
            account,
            "w_execute_signed",
            args.into_bytes(),
            NearToken::ZERO,
            Gas::from_tgas(40),
        )
        .await;
        let status = format!("{:?}", result.status);
        assert!(
            status.contains("unknown field") || status.contains("duplicate field"),
            "{status}"
        );
    }
    let args = serde_json::to_vec(&json!({"msg": msg, "proof": proof})).unwrap();
    call(
        near,
        account,
        "w_execute_signed",
        args.clone(),
        NearToken::ZERO,
        Gas::from_tgas(40),
    )
    .await
    .result()
    .unwrap();
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

    let unknown = call(
        near,
        account,
        "w_execute_extension",
        br#"{"request":{},"unused":true}"#.to_vec(),
        NearToken::from_yoctonear(1),
        Gas::from_tgas(30),
    )
    .await;
    assert!(format!("{:?}", unknown.status).contains("unknown field"));
    let auth: Result<serde_json::Value, _> = near
        .view(account, "w_resolve_auth")
        .args(json!({"path": [], "authorization": "{}", "unused": true}))
        .await;
    assert!(auth.is_err_and(|error| error.to_string().contains("unknown field")));
    duplicate_metadata(near, account).await;
}

/// Two metadata requirements for one token reject the whole request.
async fn duplicate_metadata(near: &Near, account: &AccountId) {
    let requirement = WalletOp::require_ft_metadata(near.account_id().clone(), 6, "DUP");
    let request = Request::new().internal([
        WalletOp::add_extension(account.clone()),
        requirement.clone(),
        requirement,
    ]);
    let outcome = call(
        near,
        account,
        "w_execute_extension",
        serde_json::to_vec(&json!({"request":request})).unwrap(),
        NearToken::from_yoctonear(1),
        Gas::from_tgas(30),
    )
    .await;
    let status = format!("{:?}", outcome.status);
    assert!(
        status.contains("duplicate fungible token metadata requirement"),
        "{status}"
    );
    // The operation before the duplicate is rolled back with the receipt.
    let enabled: bool = near
        .view(account, "w_is_extension_enabled")
        .args(json!({"account_id":account}))
        .await
        .unwrap();
    assert!(!enabled);
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

/// A checked promise whose gas weights overflow rejects the whole request,
/// signed or sent by an extension: no promise runs, no operation is applied
/// and the nonce of `original` (a fresh request) stays unused.
pub async fn weight_overflow<S: SignatureSchema>(
    near: &Near,
    account: &AccountId,
    original: &RequestMessage,
    signer: &impl WalletSigner<S>,
) {
    const REASON: &str = "total gas weight for checked promise exceeds";
    let token: AccountId = "missing-metadata.near".parse().unwrap();
    let sibling: AccountId = "must-not-run.near".parse().unwrap();
    let extension = WalletOp::add_extension(near.account_id().clone());
    let call_with = |name: &str, weight| {
        FunctionCall::name(name)
            .args(b"{}")
            .gas(Gas::from_tgas(1))
            .unused_gas_weight(weight)
    };
    let overflowing = |operations: Vec<WalletOp>| {
        Request::new().internal(operations).external([
            NearPromise::new(sibling.clone()).function_call(call_with("submit", 1)),
            NearPromise::new(token.clone())
                .function_call(call_with("first", u64::MAX))
                .function_call(call_with("second", 1)),
        ])
    };
    let requirement = WalletOp::require_ft_metadata(token.clone(), 6, "TEST");
    let storage = || async { near.account(account).await.unwrap().storage_usage };
    let enabled = || async {
        near.view::<bool>(account, "w_is_extension_enabled")
            .args(json!({"account_id":near.account_id()}))
            .await
            .unwrap()
    };
    let before = storage().await;

    let mut msg = original.clone();
    msg.request = overflowing(vec![extension.clone(), requirement.clone()]);
    let proof = signer.sign_request_msg(&msg).await.unwrap();
    let outcome = call(
        near,
        account,
        "w_execute_signed",
        serde_json::to_vec(&json!({"msg":msg,"proof":proof})).unwrap(),
        NearToken::ZERO,
        Gas::from_tgas(80),
    )
    .await;
    let status = format!("{:?}", outcome.status);
    assert!(status.contains(REASON), "{status}");
    assert!(!outcome.receipts_outcome.iter().any(|receipt| {
        receipt.outcome.executor_id == token || receipt.outcome.executor_id == sibling
    }));
    assert_eq!(storage().await, before);
    assert!(!enabled().await);

    // The same nonce still executes: the failed receipt committed nothing.
    msg.request = extension.into();
    let proof = signer.sign_request_msg(&msg).await.unwrap();
    call(
        near,
        account,
        "w_execute_signed",
        serde_json::to_vec(&json!({"msg":msg,"proof":proof})).unwrap(),
        NearToken::ZERO,
        Gas::from_tgas(30),
    )
    .await
    .result()
    .unwrap();
    assert!(enabled().await);

    // Removing the authorized extension must roll back along with the request.
    extension_weight_overflow(
        near,
        account,
        overflowing(vec![
            WalletOp::remove_extension(near.account_id().clone()),
            requirement,
        ]),
        &token,
        &sibling,
    )
    .await;
    assert!(enabled().await);
    let remove = Request::from(WalletOp::remove_extension(near.account_id().clone()));
    call(
        near,
        account,
        "w_execute_extension",
        serde_json::to_vec(&json!({"request":remove})).unwrap(),
        NearToken::from_yoctonear(1),
        Gas::from_tgas(30),
    )
    .await
    .result()
    .unwrap();
    assert!(!enabled().await);
}

async fn extension_weight_overflow(
    near: &Near,
    account: &AccountId,
    request: Request,
    token: &AccountId,
    sibling: &AccountId,
) {
    let state = || async {
        near.rpc()
            .view_state_all(account, b"", 100, near_kit::rpc::BlockReference::final_())
            .await
            .unwrap()
            .values
    };
    let before = state().await;
    let storage_before = near.account(account).await.unwrap().storage_usage;
    let outcome = call(
        near,
        account,
        "w_execute_extension",
        serde_json::to_vec(&json!({"request":request})).unwrap(),
        NearToken::from_yoctonear(1),
        Gas::from_tgas(80),
    )
    .await;
    let status = format!("{:?}", outcome.status);
    assert!(
        status.contains("total gas weight for checked promise exceeds"),
        "{status}"
    );
    assert!(!outcome.receipts_outcome.iter().any(|receipt| {
        &receipt.outcome.executor_id == token || &receipt.outcome.executor_id == sibling
    }));
    assert_eq!(state().await, before);
    assert_eq!(
        near.account(account).await.unwrap().storage_usage,
        storage_before
    );
}

/// Test the full metadata byte limit and receipt-creation budget using a real FT.
/// Returns a valid request that fails only in its metadata callback.
pub async fn metadata(near: &Near, account: &AccountId, ft_wasm: &[u8]) -> Request {
    let mut too_large = Request::new();
    for size in [32 * 1024, 32 * 1024 + 1] {
        let token = deploy_metadata_token(near, account, ft_wasm, size).await;
        let returned: serde_json::Value = near.view(&token, "ft_metadata").await.unwrap();
        assert_eq!(serde_json::to_vec(&returned).unwrap().len(), size);
        let promise = NearPromise::new(&token).function_call(
            FunctionCall::name("ft_transfer")
                .args(json!({"receiver_id":near.account_id(),"amount":"1"}).to_string())
                .attach_deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(10))
                .unused_gas_weight(0),
        );
        let request = Request::new()
            .internal([WalletOp::require_ft_metadata(token.clone(), 6, "BOUND")])
            .external([promise.clone(), promise]);
        let outcome = call(
            near,
            account,
            "w_execute_extension",
            serde_json::to_vec(&json!({"request":request})).unwrap(),
            NearToken::from_yoctonear(1),
            request.estimate_gas(),
        )
        .await;
        outcome.result().unwrap();
        let balance: String = near
            .view(&token, "ft_balance_of")
            .args(json!({"account_id":near.account_id()}))
            .await
            .unwrap();
        if size == 32 * 1024 {
            assert_eq!(balance, "2", "{outcome:?}");
            assert!(
                outcome
                    .receipts_outcome
                    .iter()
                    .all(|r| !format!("{:?}", r.outcome.status).contains("Failure")),
                "{outcome:?}"
            );
        } else {
            assert_eq!(balance, "0");
            assert!(
                outcome
                    .receipts_outcome
                    .iter()
                    .any(|r| format!("{:?}", r.outcome.status)
                        .contains("fungible token metadata does not match")),
                "{outcome:?}"
            );
            too_large = request;
        }
        for receipt in &outcome.receipts_outcome {
            eprintln!(
                "metadata boundary {size}: {} burnt {}",
                receipt.outcome.executor_id, receipt.outcome.gas_burnt
            );
        }
    }
    let remove = Request::from(WalletOp::remove_extension(near.account_id().clone()));
    call(
        near,
        account,
        "w_execute_extension",
        serde_json::to_vec(&json!({"request":remove})).unwrap(),
        NearToken::from_yoctonear(1),
        Gas::from_tgas(30),
    )
    .await
    .result()
    .unwrap();
    too_large
}

pub async fn failed_metadata_consumes_nonce(
    near: &Near,
    account: &AccountId,
    msg: &RequestMessage,
    proof: &str,
) {
    let args = serde_json::to_vec(&json!({"msg":msg,"proof":proof})).unwrap();
    let outcome = call(
        near,
        account,
        "w_execute_signed",
        args.clone(),
        NearToken::ZERO,
        Gas::from_tgas(200),
    )
    .await;
    outcome.result().unwrap();
    assert!(
        outcome
            .receipts_outcome
            .iter()
            .any(|r| format!("{:?}", r.outcome.status)
                .contains("fungible token metadata does not match")),
        "{outcome:?}"
    );
    let replay = call(
        near,
        account,
        "w_execute_signed",
        args,
        NearToken::ZERO,
        Gas::from_tgas(200),
    )
    .await;
    assert!(format!("{:?}", replay.status).contains("already been used"));
}

async fn deploy_metadata_token(
    near: &Near,
    account: &AccountId,
    ft_wasm: &[u8],
    size: usize,
) -> AccountId {
    let token: AccountId = format!("m{size}{}.{}", &account.as_str()[2..14], near.account_id())
        .parse()
        .unwrap();
    let mut metadata = json!({"spec":"ft-1.0.0", "name":"Metadata boundary", "symbol":"BOUND", "icon":"", "reference":null, "reference_hash":null, "decimals":6});
    let fixed = serde_json::to_vec(&metadata).unwrap().len();
    metadata["icon"] = "A".repeat(size - fixed).into();
    near.transaction(&token)
        .create_account()
        .transfer(NearToken::from_near(5))
        .deploy(ft_wasm.to_vec())
        .call("new")
        .args(json!({"owner_id": account, "total_supply":"1000000", "metadata":metadata}))
        .gas(Gas::from_tgas(80))
        .finish()
        .call("storage_deposit")
        .args(json!({"account_id":near.account_id()}))
        .deposit(NearToken::from_millinear(10))
        .gas(Gas::from_tgas(20))
        .finish()
        .wait_until::<Final>()
        .await
        .unwrap()
        .result()
        .unwrap();
    token
}
