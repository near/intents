//! Integration tests for escrow-swap with escrow-proxy using near-sandbox.
//!
//! This module tests the full flow of:
//! 1. Maker creating an escrow with tokens they want to swap
//! 2. Solver filling the escrow via the proxy with relay authorization
//! 3. Atomic token exchange between maker and solver

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;

use near_sdk::AccountId;

use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::env::Env;
use defuse::sandbox_ext::intents::ExecuteIntentsExt;
use defuse_core::amounts::Amounts;
use defuse_core::intents::auth::AuthCall;
use defuse_core::intents::tokens::{NotifyOnTransfer, Transfer};
use defuse_deadline::Deadline;
use defuse_escrow_proxy::{ProxyConfig, RolesConfig, TransferMessage as ProxyTransferMessage};
use defuse_escrow_swap::Params;
use defuse_escrow_swap::action::{
    FillAction, TransferAction, TransferMessage as EscrowTransferMessage,
};
use defuse_escrow_swap::decimal::UD128;
use defuse_sandbox::{MtExt, MtViewExt};
use defuse_sandbox_ext::{EscrowProxyExt, EscrowSwapAccountExt, TransferAuthAccountExt};
use defuse_token_id::TokenId;
use defuse_token_id::nep245::Nep245TokenId;
use defuse_transfer_auth::TransferAuthContext;
use defuse_transfer_auth::storage::{
    ContractStorage as TransferAuthStorage, StateInit as TransferAuthState,
};
use near_sdk::json_types::U128;
use near_sdk::state_init::{StateInit, StateInitV1};
use near_sdk::{GlobalContractId, NearToken};

/// Builder for creating escrow swap parameters.
/// Takes (maker, `src_token`) and (taker, `dst_token`) tuples associating actors with their tokens.
struct ParamsBuilder {
    maker: AccountId,
    src_token: TokenId,
    taker: AccountId,
    dst_token: TokenId,
    salt: Option<[u8; 32]>,
    price: Option<UD128>,
    partial_fills_allowed: Option<bool>,
    deadline: Option<Deadline>,
    refund_src_to: Option<defuse_escrow_swap::OverrideSend>,
    receive_dst_to: Option<defuse_escrow_swap::OverrideSend>,
    auth_caller: Option<AccountId>,
    protocol_fees: Option<defuse_escrow_swap::ProtocolFees>,
    integrator_fees: Option<BTreeMap<AccountId, defuse_escrow_swap::Pips>>,
}

impl ParamsBuilder {
    fn new(
        (maker, src_token): (AccountId, TokenId),
        (taker, dst_token): (AccountId, TokenId),
    ) -> Self {
        Self {
            maker,
            src_token,
            taker,
            dst_token,
            salt: None,
            price: None,
            partial_fills_allowed: None,
            deadline: None,
            refund_src_to: None,
            receive_dst_to: None,
            auth_caller: None,
            protocol_fees: None,
            integrator_fees: None,
        }
    }

    fn with_salt(mut self, salt: [u8; 32]) -> Self {
        self.salt = Some(salt);
        self
    }

    fn with_price(mut self, price: UD128) -> Self {
        self.price = Some(price);
        self
    }

    fn with_partial_fills_allowed(mut self, allowed: bool) -> Self {
        self.partial_fills_allowed = Some(allowed);
        self
    }

    fn with_deadline(mut self, deadline: Deadline) -> Self {
        self.deadline = Some(deadline);
        self
    }

    fn with_refund_src_to(mut self, refund_src_to: defuse_escrow_swap::OverrideSend) -> Self {
        self.refund_src_to = Some(refund_src_to);
        self
    }

    fn with_receive_dst_to(mut self, receive_dst_to: defuse_escrow_swap::OverrideSend) -> Self {
        self.receive_dst_to = Some(receive_dst_to);
        self
    }

    fn with_auth_caller(mut self, auth_caller: AccountId) -> Self {
        self.auth_caller = Some(auth_caller);
        self
    }

    fn with_protocol_fees(mut self, protocol_fees: defuse_escrow_swap::ProtocolFees) -> Self {
        self.protocol_fees = Some(protocol_fees);
        self
    }

    fn with_integrator_fees(
        mut self,
        integrator_fees: BTreeMap<AccountId, defuse_escrow_swap::Pips>,
    ) -> Self {
        self.integrator_fees = Some(integrator_fees);
        self
    }

    fn build(self) -> (Params, EscrowTransferMessage, EscrowTransferMessage) {
        let price = self.price.unwrap_or(UD128::ONE);
        let params = Params {
            maker: self.maker,
            src_token: self.src_token,
            dst_token: self.dst_token,
            price,
            deadline: self
                .deadline
                .unwrap_or_else(|| Deadline::timeout(Duration::from_secs(360))),
            partial_fills_allowed: self.partial_fills_allowed.unwrap_or(false),
            refund_src_to: self.refund_src_to.unwrap_or_default(),
            receive_dst_to: self.receive_dst_to.unwrap_or_default(),
            taker_whitelist: [self.taker.clone()].into(),
            protocol_fees: self.protocol_fees,
            integrator_fees: self.integrator_fees.unwrap_or_default(),
            auth_caller: self.auth_caller,
            salt: self.salt.unwrap_or([7u8; 32]),
        };
        let fund_msg = EscrowTransferMessage {
            params: params.clone(),
            action: TransferAction::Fund,
        };
        let fill_msg = EscrowTransferMessage {
            params: params.clone(),
            action: TransferAction::Fill(FillAction {
                price,
                deadline: Deadline::timeout(Duration::from_secs(120)),
                receive_src_to: defuse_escrow_swap::OverrideSend::default()
                    .receiver_id(self.taker),
            }),
        };
        (params, fund_msg, fill_msg)
    }
}

/// Test full escrow swap flow with proxy authorization
#[tokio::test]
async fn test_escrow_swap_with_proxy_full_flow() {
    let swap_amount: u128 = 100_000_000; // Fits within ft_deposit_to_root mint limit (1e9)
    let env = Env::builder().build().await;
    let (transfer_auth_global, escrow_swap_global) = futures::join!(
        env.root().deploy_transfer_auth("transfer_auth"),
        env.root().deploy_escrow_swap_global("escrow_swap"),
    );
    let (maker, solver, relay, proxy) = futures::join!(
        env.create_named_user("maker"),
        env.create_named_user("solver"),
        env.create_named_user("relay"),
        env.create_named_user("proxy"),
    );

    let (token_a_result, token_b_result) = futures::join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), swap_amount)]),
        env.create_mt_token_with_initial_balances([(solver.id().clone(), swap_amount)]),
    );
    let (_, token_a_defuse_id) = token_a_result.unwrap();
    let (_, token_b_defuse_id) = token_b_result.unwrap();

    let roles = RolesConfig {
        super_admins: HashSet::from([env.root().id().clone()]),
        admins: HashMap::new(),
        grantees: HashMap::new(),
    };
    let config = ProxyConfig {
        per_fill_contract_id: GlobalContractId::AccountId(transfer_auth_global.clone()),
        escrow_swap_contract_id: GlobalContractId::AccountId(escrow_swap_global.clone()),
        auth_contract: env.defuse.id().clone(),
        auth_collee: relay.id().clone(),
    };
    proxy
        .deploy_escrow_proxy(roles, config.clone())
        .await
        .unwrap();

    let src_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_a_defuse_id.to_string(),
    ));
    let dst_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_b_defuse_id.to_string(),
    ));
    let (escrow_params, fund_escrow_msg, fill_escrow_msg) = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        (proxy.id().clone(), dst_token),
    )
    .build();

    let fund_msg_json = serde_json::to_string(&fund_escrow_msg).unwrap();

    let escrow_raw_state = defuse_escrow_swap::ContractStorage::init_state(&escrow_params).unwrap();
    let escrow_state_init =
        StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(escrow_swap_global.clone()),
            data: escrow_raw_state,
        });
    let escorow_instance_id = escrow_state_init.derive_account_id();

    let transfer = Transfer {
        receiver_id: escorow_instance_id.clone(),
        tokens: Amounts::new([(token_a_defuse_id.clone(), swap_amount)].into()),
        memo: None,
        notification: Some(NotifyOnTransfer::new(fund_msg_json).with_state_init(escrow_state_init)),
    };

    // Maker signs and executes transfer intent
    let transfer_payload = maker
        .sign_defuse_payload_default(&env.defuse, [transfer])
        .await
        .unwrap();
    maker
        .simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    let proxy_msg = ProxyTransferMessage {
        receiver_id: escorow_instance_id.clone(),
        salt: [2u8; 32],
        msg: serde_json::to_string(&fill_escrow_msg).unwrap(),
    };
    let proxy_msg_json = serde_json::to_string(&proxy_msg).unwrap();

    let context_hash = TransferAuthContext {
        sender_id: Cow::Borrowed(solver.id().as_ref()),
        token_ids: Cow::Owned(vec![token_b_defuse_id.to_string()]),
        amounts: Cow::Owned(vec![U128(swap_amount)]),
        salt: proxy_msg.salt,
        msg: Cow::Borrowed(&proxy_msg_json),
    }
    .hash();

    let auth_state = TransferAuthState {
        escrow_contract_id: config.escrow_swap_contract_id.clone(),
        auth_contract: env.defuse.id().clone(),
        on_auth_signer: relay.id().clone(),
        authorizee: proxy.id().clone(),
        msg_hash: context_hash,
    };
    let transfer_auth_raw_state = TransferAuthStorage::init_state(auth_state.clone()).unwrap();
    let transfer_auth_state_init =
        StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(transfer_auth_global.clone()),
            data: transfer_auth_raw_state,
        });

    let auth_payload = relay
        .sign_defuse_payload_default(
            &env.defuse,
            [AuthCall {
                contract_id: transfer_auth_state_init.derive_account_id(),
                state_init: Some(transfer_auth_state_init),
                msg: String::new(),
                attached_deposit: NearToken::from_yoctonear(0),
                min_gas: None,
            }],
        )
        .await
        .unwrap();
    relay
        .simulate_and_execute_intents(env.defuse.id(), [auth_payload])
        .await
        .unwrap();

    solver
        .mt_transfer_call(
            env.defuse.id(),
            proxy.id(),
            &token_b_defuse_id.to_string(),
            swap_amount,
            None,
            &proxy_msg_json,
        )
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(maker.id(), &token_b_defuse_id.to_string())
            .await
            .unwrap(),
        swap_amount,
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(solver.id(), &token_a_defuse_id.to_string())
            .await
            .unwrap(),
        swap_amount,
        "Solver should have received token-a"
    );
}

/// Test direct escrow swap flow without proxy authorization
/// Simpler scenario with just maker, solver, and globally deployed escrow contract
#[tokio::test]
async fn test_escrow_swap_direct_fill() {
    let swap_amount: u128 = 100_000_000;
    let env = Env::builder().build().await;

    let escrow_swap_global = env.root().deploy_escrow_swap_global("escrow_swap").await;

    let (maker, solver) = futures::join!(
        env.create_named_user("maker"),
        env.create_named_user("solver"),
    );

    let (token_a_result, token_b_result) = futures::join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), swap_amount)]),
        env.create_mt_token_with_initial_balances([(solver.id().clone(), swap_amount)]),
    );
    let (_, token_a_defuse_id) = token_a_result.unwrap();
    let (_, token_b_defuse_id) = token_b_result.unwrap();

    let src_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_a_defuse_id.to_string(),
    ));
    let dst_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        token_b_defuse_id.to_string(),
    ));
    let (escrow_params, fund_escrow_msg, fill_escrow_msg) = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        (solver.id().clone(), dst_token),
    )
    .build();

    let fund_msg_json = serde_json::to_string(&fund_escrow_msg).unwrap();

    let escrow_raw_state = defuse_escrow_swap::ContractStorage::init_state(&escrow_params).unwrap();
    let escrow_state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(escrow_swap_global.clone()),
        data: escrow_raw_state,
    });
    let escrow_instance_id = escrow_state_init.derive_account_id();

    let transfer = Transfer {
        receiver_id: escrow_instance_id.clone(),
        tokens: Amounts::new([(token_a_defuse_id.clone(), swap_amount)].into()),
        memo: None,
        notification: Some(NotifyOnTransfer::new(fund_msg_json).with_state_init(escrow_state_init)),
    };

    // Maker signs and executes transfer intent to fund the escrow
    let transfer_payload = maker
        .sign_defuse_payload_default(&env.defuse, [transfer])
        .await
        .unwrap();
    maker
        .simulate_and_execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    // Solver fills the escrow directly via mt_transfer_call
    let fill_msg_json = serde_json::to_string(&fill_escrow_msg).unwrap();
    solver
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_instance_id,
            &token_b_defuse_id.to_string(),
            swap_amount,
            None,
            &fill_msg_json,
        )
        .await
        .unwrap();

    // Verify maker received token_b
    assert_eq!(
        env.defuse
            .mt_balance_of(maker.id(), &token_b_defuse_id.to_string())
            .await
            .unwrap(),
        swap_amount,
        "Maker should have received token-b"
    );

    // Verify solver received token_a
    assert_eq!(
        env.defuse
            .mt_balance_of(solver.id(), &token_a_defuse_id.to_string())
            .await
            .unwrap(),
        swap_amount,
        "Solver should have received token-a"
    );
}
