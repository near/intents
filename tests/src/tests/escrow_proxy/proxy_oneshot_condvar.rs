use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::env::Env;
use defuse_core::token_id::TokenId;
use defuse_core::token_id::nep141::Nep141TokenId;
use defuse_escrow_proxy::CondVarContext;
use defuse_escrow_proxy::{ProxyConfig, TransferMessage};
use defuse_oneshot_condvar::storage::{Config as CondVarConfig, ContractStorage};
use defuse_sandbox::extensions::storage_management::StorageManagementExt;
use defuse_sandbox::{
    EscrowProxyExt, FnCallBuilder, FtExt, FtViewExt, MtExt, MtReceiverStubExt, MtViewExt,
    OneshotCondVarExt,
};
use multi_token_receiver_stub::{FTReceiverMode, MTReceiverMode};
use near_sdk::AccountId;
use near_sdk::serde_json;
use near_sdk::{
    Gas, GlobalContractId, NearToken,
    json_types::U128,
    state_init::{StateInit, StateInitV1},
};

/// Derive the oneshot-condvar instance account ID from its state
pub fn derive_oneshot_condvar_account_id(
    global_contract_id: &GlobalContractId,
    state: &CondVarConfig,
) -> AccountId {
    let raw_state = ContractStorage::init_state(state.clone()).unwrap();
    let state_init = StateInit::V1(StateInitV1 {
        code: global_contract_id.clone(),
        data: raw_state,
    });
    state_init.derive_account_id()
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_proxy_returns_funds_on_timeout_of_authorization() {
    let env = Env::builder().build().await;
    let (condvar_global, mt_receiver_global, escrow_proxy_global) = futures::join!(
        env.root().deploy_oneshot_condvar("global_transfer_auth"),
        env.root()
            .deploy_mt_receiver_stub_global("mt_receiver_global"),
        env.root().deploy_escrow_proxy_global("escrow_proxy_global"),
    );
    let mt_receiver_instance = env
        .root()
        .deploy_mt_receiver_stub_instance(mt_receiver_global.clone(), BTreeMap::default())
        .await;

    let (solver, relay) = futures::join!(
        env.create_named_user("solver"),
        env.create_named_user("relay"),
    );

    let owner_id = env.root().sub_account("proxy_owner").unwrap().id().clone();
    // Setup proxy
    let config = ProxyConfig {
        owner_id,
        oneshot_condvar_global_id: GlobalContractId::AccountId(condvar_global.clone()),
        on_auth_caller: env.defuse.id().clone(),
        notifier_id: relay.id().clone(),
    };

    let proxy_id = env
        .root()
        .deploy_escrow_proxy_instance(escrow_proxy_global, config)
        .await;

    // Create MT token with initial balance for solver
    let initial_amount = 1_000_000u128;
    let (_, token_id) = env
        .create_mt_token_with_initial_balances([(solver.id().clone(), initial_amount)])
        .await
        .unwrap();

    let transfer_msg = TransferMessage {
        receiver_id: mt_receiver_instance.clone(),
        salt: Some([1u8; 32]),
        msg: String::new(),
    };
    let msg_json = serde_json::to_string(&transfer_msg).unwrap();
    let token_id_str = token_id.to_string();
    let (transfer_result, ()) = futures::join!(
        solver.mt_transfer_call(
            env.defuse.id(),
            &proxy_id,
            &token_id_str,
            initial_amount / 2,
            None,
            &msg_json,
        ),
        env.sandbox().fast_forward(250)
    );

    assert_eq!(
        transfer_result.unwrap(),
        0,
        "Used amount should be 0 when transfer times out and refunds"
    );

    assert_eq!(
        initial_amount,
        env.defuse
            .mt_balance_of(solver.id(), &token_id_str)
            .await
            .unwrap(),
        "Solver balance should be unchanged after timeout refund"
    );
}

/// Test that transfer succeeds when relay authorizes via on_auth call
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_transfer_authorized_by_relay() {
    let env = Env::builder().build().await;

    let (condvar_global, mt_receiver_global, escrow_proxy_global) = futures::join!(
        env.root().deploy_oneshot_condvar("global_transfer_auth"),
        env.root()
            .deploy_mt_receiver_stub_global("mt_receiver_global"),
        env.root().deploy_escrow_proxy_global("escrow_proxy_global"),
    );

    let (solver, relay) = futures::join!(
        env.create_named_user("solver"),
        env.create_named_user("relay"),
    );

    let owner_id = env.root().sub_account("proxy_owner").unwrap().id().clone();
    let config = ProxyConfig {
        owner_id,
        oneshot_condvar_global_id: GlobalContractId::AccountId(condvar_global.clone()),
        on_auth_caller: env.root().id().clone(),
        notifier_id: relay.id().clone(),
    };

    let proxy_id = env
        .root()
        .deploy_escrow_proxy_instance(escrow_proxy_global, config.clone())
        .await;

    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(mt_receiver_global.clone()),
        data: BTreeMap::new(),
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();

    env.root()
        .tx(escrow_instance_id.clone())
        .state_init(mt_receiver_global.clone(), BTreeMap::new())
        .transfer(NearToken::from_yoctonear(1))
        .await
        .unwrap();

    let initial_balance: u128 = 1_000_000;
    let (_, token_id) = env
        .create_mt_token_with_initial_balances([(solver.id().clone(), initial_balance)])
        .await
        .unwrap();

    let initial_solver_balance = env
        .defuse
        .mt_balance_of(solver.id(), &token_id.to_string())
        .await
        .unwrap();

    let inner_msg_json = serde_json::to_string(&MTReceiverMode::AcceptAll).unwrap();

    let transfer_msg = TransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: Some([2u8; 32]), // Different salt from timeout test
        msg: inner_msg_json,
    };
    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    let proxy_transfer_amount: u128 = 100_000;

    let context_hash = CondVarContext {
        sender_id: Cow::Borrowed(solver.id()),
        token_ids: Cow::Owned(vec![token_id.to_string()]),
        amounts: Cow::Owned(vec![U128(proxy_transfer_amount)]),
        salt: transfer_msg.salt.unwrap_or_default(),
        msg: Cow::Borrowed(&msg_json),
    }
    .hash();

    let auth_state = CondVarConfig {
        on_auth_caller: config.on_auth_caller.clone(),
        notifier_id: config.notifier_id.clone(),
        waiter: proxy_id.clone(),
        salt: context_hash,
    };
    let condvar_instance_id = derive_oneshot_condvar_account_id(
        &GlobalContractId::AccountId(condvar_global.clone()),
        &auth_state,
    );

    let token_id_str = token_id.to_string();

    let raw_state = ContractStorage::init_state(auth_state).unwrap();

    let (_transfer_result, _auth_result) = futures::join!(
        solver.mt_transfer_call(
            env.defuse.id(),
            &proxy_id,
            &token_id_str,
            proxy_transfer_amount,
            None,
            &msg_json,
        ),
        async {
            env.root()
                .tx(condvar_instance_id.clone())
                .state_init(condvar_global.clone(), raw_state)
                .function_call(
                    FnCallBuilder::new("on_auth")
                        .json_args(serde_json::json!({
                            "signer_id": relay.id(),
                            "msg": "",
                        }))
                        .with_gas(Gas::from_tgas(50)),
                )
                .await
                .unwrap();
        }
    );

    // Verify solver balance decreased
    let final_solver_balance = env
        .defuse
        .mt_balance_of(solver.id(), &token_id_str)
        .await
        .unwrap();

    assert_eq!(
        initial_solver_balance - proxy_transfer_amount,
        final_solver_balance,
        "Solver balance should decrease by transferred amount"
    );

    // Verify escrow instance received the tokens
    let escrow_balance = env
        .defuse
        .mt_balance_of(&escrow_instance_id, &token_id_str)
        .await
        .unwrap();

    assert_eq!(
        escrow_balance, proxy_transfer_amount,
        "Escrow instance should have received the transferred tokens"
    );
}

/// Test that FT transfer succeeds when relay authorizes via on_auth call
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_ft_transfer_authorized_by_relay() {
    let env = Env::builder().build().await;

    let (condvar_global, ft_receiver_global, escrow_proxy_global) = futures::join!(
        env.root().deploy_oneshot_condvar("global_transfer_auth"),
        env.root()
            .deploy_mt_receiver_stub_global("ft_receiver_global"),
        env.root().deploy_escrow_proxy_global("escrow_proxy_global"),
    );

    let (solver, relay) = futures::join!(
        env.create_named_user("solver"),
        env.create_named_user("relay"),
    );

    let owner_id = env.root().sub_account("proxy_owner").unwrap().id().clone();
    let config = ProxyConfig {
        owner_id,
        oneshot_condvar_global_id: GlobalContractId::AccountId(condvar_global.clone()),
        on_auth_caller: env.root().id().clone(),
        notifier_id: relay.id().clone(),
    };

    let proxy_id = env
        .root()
        .deploy_escrow_proxy_instance(escrow_proxy_global, config.clone())
        .await;

    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(ft_receiver_global.clone()),
        data: BTreeMap::new(),
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();

    env.root()
        .tx(escrow_instance_id.clone())
        .state_init(ft_receiver_global.clone(), BTreeMap::new())
        .transfer(NearToken::from_yoctonear(1))
        .await
        .unwrap();

    let initial_balance: u128 = 1_000_000;
    let (ft_token, _) = env
        .create_ft_token_with_initial_balances([
            (solver.id().clone(), initial_balance),
            (proxy_id.clone(), 0),
            (escrow_instance_id.clone(), 0),
        ])
        .await
        .unwrap();

    let inner_msg_json = serde_json::to_string(&FTReceiverMode::AcceptAll).unwrap();

    let transfer_msg = TransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: Some([3u8; 32]), // Different salt from other tests
        msg: inner_msg_json,
    };
    let msg_json = serde_json::to_string(&transfer_msg).unwrap();

    let proxy_transfer_amount: u128 = 100_000;

    // Use canonical TokenId::Nep141 format to match what the proxy computes
    let token_id = TokenId::from(Nep141TokenId::new(ft_token.id().clone()));
    let context_hash = CondVarContext {
        sender_id: Cow::Borrowed(solver.id()),
        token_ids: Cow::Owned(vec![token_id.to_string()]),
        amounts: Cow::Owned(vec![U128(proxy_transfer_amount)]),
        salt: transfer_msg.salt.unwrap_or_default(),
        msg: Cow::Borrowed(&msg_json),
    }
    .hash();

    let auth_state = CondVarConfig {
        on_auth_caller: config.on_auth_caller.clone(),
        notifier_id: config.notifier_id.clone(),
        waiter: proxy_id.clone(),
        salt: context_hash,
    };
    let condvar_instance_id = derive_oneshot_condvar_account_id(
        &GlobalContractId::AccountId(condvar_global.clone()),
        &auth_state,
    );

    let raw_state = ContractStorage::init_state(auth_state).unwrap();
    let (_transfer_result, _auth_result) = futures::join!(
        solver.ft_transfer_call(
            ft_token.id(),
            &proxy_id,
            proxy_transfer_amount,
            None,
            &msg_json,
        ),
        async {
            env.root()
                .tx(condvar_instance_id.clone())
                .state_init(condvar_global.clone(), raw_state)
                .function_call(
                    FnCallBuilder::new("on_auth")
                        .json_args(serde_json::json!({
                            "signer_id": relay.id(),
                            "msg": "",
                        }))
                        .with_gas(Gas::from_tgas(50)),
                )
                .await
                .unwrap();
        }
    );

    // Verify solver balance decreased
    let final_solver_balance = ft_token.ft_balance_of(solver.id()).await.unwrap();

    assert_eq!(
        initial_balance - proxy_transfer_amount,
        final_solver_balance,
        "Solver balance should decrease by transferred amount"
    );

    // Verify escrow instance received the tokens
    let escrow_balance = ft_token.ft_balance_of(&escrow_instance_id).await.unwrap();

    assert_eq!(
        escrow_balance, proxy_transfer_amount,
        "Escrow instance should have received the transferred tokens"
    );
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_proxy_with_ft_transfer() {
    use std::time::Duration;

    use crate::utils::escrow_builders::{FillMessageBuilder, FundMessageBuilder, ParamsBuilder};
    use defuse_sandbox::EscrowSwapExt;

    let env = Env::builder().build().await;

    let (condvar_global, escrow_swap_global, escrow_proxy_global) = futures::join!(
        env.root().deploy_oneshot_condvar("global_transfer_auth"),
        env.root().deploy_escrow_swap_global("escrow_swap"),
        env.root().deploy_escrow_proxy_global("escrow_proxy_global"),
    );

    let (maker, solver, relay) = futures::join!(
        env.create_named_user("maker"),
        env.create_named_user("solver"),
        env.create_named_user("relay"),
    );

    let owner_id = env.root().sub_account("proxy_owner").unwrap().id().clone();
    let config = ProxyConfig {
        owner_id,
        oneshot_condvar_global_id: GlobalContractId::AccountId(condvar_global.clone()),
        on_auth_caller: env.root().id().clone(),
        notifier_id: relay.id().clone(),
    };

    let proxy_id = env
        .root()
        .deploy_escrow_proxy_instance(escrow_proxy_global, config.clone())
        .await;

    let swap_amount: u128 = 100_000;
    let ((src_ft, _), (dst_ft, _)) = futures::try_join!(
        env.create_ft_token_with_initial_balances([
            (maker.id().clone(), swap_amount),
            (proxy_id.clone(), 0),
        ]),
        env.create_ft_token_with_initial_balances([
            (solver.id().clone(), swap_amount),
            (proxy_id.clone(), 0),
            (maker.id().clone(), 0),
        ]),
    )
    .unwrap();

    let src_token = TokenId::from(Nep141TokenId::new(src_ft.id().clone()));
    let dst_token = TokenId::from(Nep141TokenId::new(dst_ft.id().clone()));

    let escrow_params = ParamsBuilder::new(
        (maker.id().clone(), src_token.clone()),
        ([proxy_id.clone()], dst_token.clone()),
    )
    .build();

    let fund_escrow_msg = FundMessageBuilder::new(escrow_params.clone()).build();
    let fill_escrow_msg = FillMessageBuilder::new(escrow_params.clone())
        .with_deadline(defuse_core::Deadline::timeout(Duration::from_secs(120)))
        .build();

    let escrow_raw_state = defuse_escrow_swap::ContractStorage::init_state(&escrow_params).unwrap();
    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(escrow_swap_global.clone()),
        data: escrow_raw_state.clone(),
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();

    env.root()
        .tx(escrow_instance_id.clone())
        .state_init(escrow_swap_global.clone(), escrow_raw_state)
        .transfer(NearToken::from_yoctonear(1))
        .await
        .unwrap();

    let (src_storage, dst_storage) = futures::join!(
        maker.storage_deposit(
            src_ft.id(),
            Some(escrow_instance_id.as_ref()),
            NearToken::from_near(1)
        ),
        solver.storage_deposit(
            dst_ft.id(),
            Some(escrow_instance_id.as_ref()),
            NearToken::from_near(1)
        ),
    );
    src_storage.unwrap();
    dst_storage.unwrap();

    let fund_msg_json = serde_json::to_string(&fund_escrow_msg).unwrap();
    maker
        .ft_transfer_call(
            src_ft.id(),
            &escrow_instance_id,
            swap_amount,
            None,
            &fund_msg_json,
        )
        .await
        .unwrap();

    let escrow_src_balance = src_ft.ft_balance_of(&escrow_instance_id).await.unwrap();
    assert_eq!(
        escrow_src_balance, swap_amount,
        "Escrow should have src tokens"
    );

    let proxy_msg = TransferMessage {
        receiver_id: escrow_instance_id.clone(),
        salt: Some([4u8; 32]),
        msg: serde_json::to_string(&fill_escrow_msg).unwrap(),
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg).unwrap();

    let context_hash = CondVarContext {
        sender_id: Cow::Borrowed(solver.id()),
        token_ids: Cow::Owned(vec![dst_token.to_string()]), // "nep141:<contract_id>"
        amounts: Cow::Owned(vec![U128(swap_amount)]),
        salt: proxy_msg.salt.unwrap_or_default(),
        msg: Cow::Borrowed(&proxy_msg_json),
    }
    .hash();

    let auth_state = CondVarConfig {
        on_auth_caller: config.on_auth_caller.clone(),
        notifier_id: config.notifier_id.clone(),
        waiter: proxy_id.clone(),
        salt: context_hash,
    };
    let condvar_instance_id = derive_oneshot_condvar_account_id(
        &GlobalContractId::AccountId(condvar_global.clone()),
        &auth_state,
    );

    let raw_state = ContractStorage::init_state(auth_state).unwrap();

    let (_transfer_result, _auth_result) = futures::join!(
        solver.ft_transfer_call(dst_ft.id(), &proxy_id, swap_amount, None, &proxy_msg_json,),
        async {
            env.root()
                .tx(condvar_instance_id.clone())
                .state_init(condvar_global.clone(), raw_state)
                .function_call(
                    FnCallBuilder::new("on_auth")
                        .json_args(serde_json::json!({
                            "signer_id": relay.id(),
                            "msg": "",
                        }))
                        .with_gas(Gas::from_tgas(50)),
                )
                .await
                .unwrap();
        }
    );

    let maker_dst_balance = dst_ft.ft_balance_of(maker.id()).await.unwrap();
    assert_eq!(maker_dst_balance, swap_amount);

    let proxy_src_balance = src_ft.ft_balance_of(&proxy_id).await.unwrap();
    assert_eq!(proxy_src_balance, swap_amount);
}
