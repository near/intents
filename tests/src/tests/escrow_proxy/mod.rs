//! Integration tests for the escrow-proxy contract.
//!
//! This module tests the SolverBus proxy contract which receives tokens via `mt_on_transfer`
//! and forwards them to an escrow account after verifying a relay signature.

use crate::{
    tests::defuse::{
        accounts::AccountManagerExt,
        env::{Env, get_account_public_key},
    },
    utils::{account::AccountExt, mt::MtExt, read_wasm},
};
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse_escrow_proxy::{EscrowParams, FillAuthorization, Price, TransferMessage};
use defuse_token_id::TokenId as ProxyTokenId;
use near_sdk::json_types::U128;
use serde_json::json;
use std::sync::LazyLock;

static ESCROW_PROXY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/escrow-proxy/defuse_escrow_proxy"));

/// Helper trait for signing FillAuthorization messages
trait FillAuthorizationSigner {
    fn sign_fill_authorization(&self, auth: &FillAuthorization) -> defuse_crypto::Signature;
}

impl FillAuthorizationSigner for near_workspaces::Account {
    fn sign_fill_authorization(&self, auth: &FillAuthorization) -> defuse_crypto::Signature {
        let secret_key: near_crypto::SecretKey = self.secret_key().to_string().parse().unwrap();
        let hash = auth.hash();

        match secret_key.sign(&hash) {
            near_crypto::Signature::ED25519(sig) => {
                defuse_crypto::Signature::Ed25519(sig.to_bytes())
            }
            _ => unreachable!(),
        }
    }
}

/// Extract Ed25519 public key from an account
fn get_ed25519_public_key(account: &near_workspaces::Account) -> defuse_crypto::PublicKey {
    let pk_str = account.secret_key().public_key().to_string();
    // Format is "ed25519:BASE58" - parse it
    pk_str.parse().unwrap()
}

#[tokio::test]
async fn escrow_proxy_forwards_tokens_to_escrow() {
    // 1. Build environment with defuse contract
    let env = Env::builder().create_unique_users().build().await;

    // 2. Create test users and token
    // - solver: sends tokens through the proxy
    // - escrow: receives forwarded tokens (simulating an escrow account)
    // - relay: signs authorizations (its public key is stored in proxy)
    let (solver, escrow_receiver, relay, ft) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token()
    );

    // 3. Set up storage deposits for all accounts
    env.initial_ft_storage_deposit(
        vec![solver.id(), escrow_receiver.id()],
        vec![&ft],
    )
    .await;

    // 4. Deposit FT to solver's account in defuse (wrapping tokens)
    let deposit_amount = 10_000u128;
    env.defuse_ft_deposit_to(&ft, deposit_amount, solver.id(), None)
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    // Verify solver has tokens in defuse
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), solver.id(), &ft_id.to_string())
            .await
            .unwrap(),
        deposit_amount,
        "Solver should have deposited tokens in defuse"
    );

    // 5. Deploy escrow-proxy contract with relay's public key
    let relay_public_key = get_ed25519_public_key(&relay);
    let proxy = env
        .deploy_contract("escrow-proxy", &ESCROW_PROXY_WASM)
        .await
        .unwrap();

    // Initialize the proxy contract
    proxy
        .call("new")
        .args_json(json!({
            "relay_public_key": relay_public_key,
            "owner_id": env.id(),
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    // 6. Register proxy and escrow_receiver's public keys in defuse
    // (so they can receive tokens and execute operations)
    proxy
        .as_account()
        .add_public_key(env.defuse.id(), get_account_public_key(proxy.as_account()))
        .await
        .unwrap();

    // 7. Build the transfer message with relay signature
    let transfer_amount = 5_000u128;
    let deadline_ns = u128::from(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos())
        + 120_000_000_000u128; // 2 minutes from now

    let fill_authorization = FillAuthorization {
        escrow: escrow_receiver.id().clone(),
        price: Price {
            numerator: U128(1),
            denominator: U128(1),
        },
        amount: U128(transfer_amount),
        token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
        receive_src_to: None,
        deadline: U128(deadline_ns),
        nonce: 1,
    };

    // Sign the authorization with relay's key
    let signature = relay.sign_fill_authorization(&fill_authorization);

    let escrow_params = EscrowParams {
        maker: solver.id().clone(),
        src_token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
        dst_token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
    };

    let transfer_msg = TransferMessage {
        authorization: fill_authorization,
        escrow_params,
        signature,
    };

    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    // 8. Transfer tokens from solver to proxy via mt_transfer_call
    // The proxy will forward tokens to escrow_receiver
    let result = solver
        .mt_transfer_call(
            env.defuse.id(),
            proxy.id(),
            &ft_id.to_string(),
            transfer_amount,
            None,
            None,
            msg_json,
        )
        .await;

    // The call should succeed
    assert!(result.is_ok(), "mt_transfer_call should succeed: {result:?}");

    // 9. Verify final balances
    // Solver should have original amount minus transferred amount
    let solver_balance = env
        .mt_contract_balance_of(env.defuse.id(), solver.id(), &ft_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        solver_balance,
        deposit_amount - transfer_amount,
        "Solver should have {0} tokens left (deposited {deposit_amount}, transferred {transfer_amount})",
        deposit_amount - transfer_amount
    );

    // Escrow receiver should have received the tokens
    let escrow_balance = env
        .mt_contract_balance_of(env.defuse.id(), escrow_receiver.id(), &ft_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        escrow_balance, transfer_amount,
        "Escrow receiver should have {transfer_amount} tokens"
    );

    // Proxy should have no tokens (it forwarded everything)
    let proxy_balance = env
        .mt_contract_balance_of(env.defuse.id(), proxy.id(), &ft_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        proxy_balance, 0,
        "Proxy should have no tokens after forwarding"
    );
}

#[tokio::test]
async fn escrow_proxy_refunds_on_invalid_signature() {
    // Test that tokens are refunded when signature verification fails
    let env = Env::builder().create_unique_users().build().await;

    let (solver, escrow_receiver, relay, wrong_signer, ft) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![solver.id(), escrow_receiver.id()], vec![&ft])
        .await;

    let deposit_amount = 10_000u128;
    env.defuse_ft_deposit_to(&ft, deposit_amount, solver.id(), None)
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    // Deploy proxy with relay's public key
    let relay_public_key = get_ed25519_public_key(&relay);
    let proxy = env
        .deploy_contract("escrow-proxy-invalid-sig", &ESCROW_PROXY_WASM)
        .await
        .unwrap();

    proxy
        .call("new")
        .args_json(json!({
            "relay_public_key": relay_public_key,
            "owner_id": env.id(),
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    proxy
        .as_account()
        .add_public_key(env.defuse.id(), get_account_public_key(proxy.as_account()))
        .await
        .unwrap();

    let transfer_amount = 5_000u128;
    let deadline_ns = u128::from(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos())
        + 120_000_000_000u128;

    let fill_authorization = FillAuthorization {
        escrow: escrow_receiver.id().clone(),
        price: Price {
            numerator: U128(1),
            denominator: U128(1),
        },
        amount: U128(transfer_amount),
        token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
        receive_src_to: None,
        deadline: U128(deadline_ns),
        nonce: 1,
    };

    // Sign with WRONG signer (not the relay whose key is in proxy)
    let signature = wrong_signer.sign_fill_authorization(&fill_authorization);

    let escrow_params = EscrowParams {
        maker: solver.id().clone(),
        src_token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
        dst_token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
    };

    let transfer_msg = TransferMessage {
        authorization: fill_authorization,
        escrow_params,
        signature,
    };

    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    // Try to transfer - should be refunded due to invalid signature
    let result = solver
        .mt_transfer_call(
            env.defuse.id(),
            proxy.id(),
            &ft_id.to_string(),
            transfer_amount,
            None,
            None,
            msg_json,
        )
        .await;

    assert!(result.is_ok(), "Transfer should complete (with refund)");

    // Solver should still have all tokens (refunded)
    let solver_balance = env
        .mt_contract_balance_of(env.defuse.id(), solver.id(), &ft_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        solver_balance, deposit_amount,
        "Solver should have all tokens back (invalid signature caused refund)"
    );

    // Escrow receiver should have no tokens
    let escrow_balance = env
        .mt_contract_balance_of(env.defuse.id(), escrow_receiver.id(), &ft_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        escrow_balance, 0,
        "Escrow receiver should have no tokens (transfer was rejected)"
    );
}

#[tokio::test]
async fn escrow_proxy_refunds_on_expired_deadline() {
    // Test that tokens are refunded when deadline has passed
    let env = Env::builder().create_unique_users().build().await;

    let (solver, escrow_receiver, relay, ft) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![solver.id(), escrow_receiver.id()], vec![&ft])
        .await;

    let deposit_amount = 10_000u128;
    env.defuse_ft_deposit_to(&ft, deposit_amount, solver.id(), None)
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    let relay_public_key = get_ed25519_public_key(&relay);
    let proxy = env
        .deploy_contract("escrow-proxy-expired", &ESCROW_PROXY_WASM)
        .await
        .unwrap();

    proxy
        .call("new")
        .args_json(json!({
            "relay_public_key": relay_public_key,
            "owner_id": env.id(),
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    proxy
        .as_account()
        .add_public_key(env.defuse.id(), get_account_public_key(proxy.as_account()))
        .await
        .unwrap();

    let transfer_amount = 5_000u128;
    // Set deadline in the PAST (already expired)
    let deadline_ns = 1u128; // Very old timestamp

    let fill_authorization = FillAuthorization {
        escrow: escrow_receiver.id().clone(),
        price: Price {
            numerator: U128(1),
            denominator: U128(1),
        },
        amount: U128(transfer_amount),
        token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
        receive_src_to: None,
        deadline: U128(deadline_ns),
        nonce: 1,
    };

    let signature = relay.sign_fill_authorization(&fill_authorization);

    let escrow_params = EscrowParams {
        maker: solver.id().clone(),
        src_token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
        dst_token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
    };

    let transfer_msg = TransferMessage {
        authorization: fill_authorization,
        escrow_params,
        signature,
    };

    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    let result = solver
        .mt_transfer_call(
            env.defuse.id(),
            proxy.id(),
            &ft_id.to_string(),
            transfer_amount,
            None,
            None,
            msg_json,
        )
        .await;

    assert!(result.is_ok(), "Transfer should complete (with refund)");

    // Solver should still have all tokens (refunded due to expired deadline)
    let solver_balance = env
        .mt_contract_balance_of(env.defuse.id(), solver.id(), &ft_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        solver_balance, deposit_amount,
        "Solver should have all tokens back (expired deadline caused refund)"
    );

    // Escrow receiver should have no tokens
    let escrow_balance = env
        .mt_contract_balance_of(env.defuse.id(), escrow_receiver.id(), &ft_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        escrow_balance, 0,
        "Escrow receiver should have no tokens (transfer was rejected)"
    );
}

#[tokio::test]
async fn escrow_proxy_refunds_on_amount_mismatch() {
    // Test that tokens are refunded when transferred amount doesn't match authorization
    let env = Env::builder().create_unique_users().build().await;

    let (solver, escrow_receiver, relay, ft) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![solver.id(), escrow_receiver.id()], vec![&ft])
        .await;

    let deposit_amount = 10_000u128;
    env.defuse_ft_deposit_to(&ft, deposit_amount, solver.id(), None)
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    let relay_public_key = get_ed25519_public_key(&relay);
    let proxy = env
        .deploy_contract("escrow-proxy-mismatch", &ESCROW_PROXY_WASM)
        .await
        .unwrap();

    proxy
        .call("new")
        .args_json(json!({
            "relay_public_key": relay_public_key,
            "owner_id": env.id(),
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    proxy
        .as_account()
        .add_public_key(env.defuse.id(), get_account_public_key(proxy.as_account()))
        .await
        .unwrap();

    let authorized_amount = 5_000u128;
    let actual_transfer_amount = 3_000u128; // Different from authorized!

    let deadline_ns = u128::from(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos())
        + 120_000_000_000u128;

    let fill_authorization = FillAuthorization {
        escrow: escrow_receiver.id().clone(),
        price: Price {
            numerator: U128(1),
            denominator: U128(1),
        },
        amount: U128(authorized_amount), // Authorization says 5000
        token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
        receive_src_to: None,
        deadline: U128(deadline_ns),
        nonce: 1,
    };

    let signature = relay.sign_fill_authorization(&fill_authorization);

    let escrow_params = EscrowParams {
        maker: solver.id().clone(),
        src_token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
        dst_token: ProxyTokenId::Nep141(defuse_token_id::nep141::Nep141TokenId::new(ft.clone())),
    };

    let transfer_msg = TransferMessage {
        authorization: fill_authorization,
        escrow_params,
        signature,
    };

    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    // Transfer 3000, but authorization says 5000
    let result = solver
        .mt_transfer_call(
            env.defuse.id(),
            proxy.id(),
            &ft_id.to_string(),
            actual_transfer_amount,
            None,
            None,
            msg_json,
        )
        .await;

    assert!(result.is_ok(), "Transfer should complete (with refund)");

    // Solver should still have all tokens (refunded due to amount mismatch)
    let solver_balance = env
        .mt_contract_balance_of(env.defuse.id(), solver.id(), &ft_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        solver_balance, deposit_amount,
        "Solver should have all tokens back (amount mismatch caused refund)"
    );

    // Escrow receiver should have no tokens
    let escrow_balance = env
        .mt_contract_balance_of(env.defuse.id(), escrow_receiver.id(), &ft_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        escrow_balance, 0,
        "Escrow receiver should have no tokens (transfer was rejected)"
    );
}
