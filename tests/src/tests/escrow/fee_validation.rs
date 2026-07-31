use defuse_fees::Pips;
use defuse_sandbox::{
    extensions::{
        defuse::{
            core::{
                Timestamp,
                intents::tokens::NotifyOnTransfer,
                token_id::{TokenId, nep141::Nep141TokenId, nep245::Nep245TokenId},
            },
            tokens::{DepositAction, DepositMessage},
        },
        escrow::{
            Escrow,
            contract::{
                ContractStorage, Error, OverrideSend, Params, ProtocolFees,
                action::{FillAction, TransferAction, TransferMessage},
            },
        },
        mt::{Mt, MtBalanceOfArgs},
    },
    kit::{AccountId, Near, StateInit, StateInitV1},
};
use near_sdk::json_types::U128;
use rstest::rstest;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use crate::tests::escrow::helpers::{Env, env};

const SRC_TOKEN: &str = "src.token.near";
const DST_TOKEN: &str = "dst.token.near";
const MAKER: &str = "maker.near";
const COLLECTOR1: &str = "collector1.near";
const COLLECTOR2: &str = "collector2.near";

#[test]
fn test_excessive_protocol_fee_does_not_pass_validation() {
    let src_token: TokenId = Nep141TokenId::new(SRC_TOKEN.parse::<AccountId>().unwrap()).into();
    let dst_token: TokenId = Nep141TokenId::new(DST_TOKEN.parse::<AccountId>().unwrap()).into();

    let params = Params {
        maker: MAKER.parse().unwrap(),
        src_token,
        dst_token,
        price: "1".parse().unwrap(),
        deadline: Timestamp::now() + Duration::from_mins(1),
        partial_fills_allowed: false,
        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        taker_whitelist: BTreeSet::default(),
        protocol_fees: Some(ProtocolFees {
            fee: Params::MAX_FEE + Pips::ONE_PERCENT,
            surplus: Pips::ZERO,
            collector: COLLECTOR1.parse().unwrap(),
        }),
        integrator_fees: BTreeMap::default(),
        auth_caller: None,
        salt: [0; 32],
    };

    let result = ContractStorage::init_state(&params);
    assert!(matches!(result, Err(Error::ExcessiveFees)));
}

#[test]
fn test_excessive_integrator_fee_does_not_pass_validation() {
    let src_token: TokenId = Nep141TokenId::new(SRC_TOKEN.parse::<AccountId>().unwrap()).into();
    let dst_token: TokenId = Nep141TokenId::new(DST_TOKEN.parse::<AccountId>().unwrap()).into();

    let params = Params {
        maker: MAKER.parse().unwrap(),
        src_token,
        dst_token,
        price: "1".parse().unwrap(),
        deadline: Timestamp::now() + Duration::from_mins(1),
        partial_fills_allowed: false,
        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        taker_whitelist: BTreeSet::default(),
        protocol_fees: None,
        integrator_fees: [(
            COLLECTOR1.parse::<AccountId>().unwrap(),
            Params::MAX_FEE + Pips::ONE_PERCENT,
        )]
        .into(),
        auth_caller: None,
        salt: [0; 32],
    };

    let result = ContractStorage::init_state(&params);
    assert!(matches!(result, Err(Error::ExcessiveFees)));
}

#[test]
fn test_excessive_integrator_fees_sum_does_not_pass_validation() {
    let src_token: TokenId = Nep141TokenId::new(SRC_TOKEN.parse::<AccountId>().unwrap()).into();
    let dst_token: TokenId = Nep141TokenId::new(DST_TOKEN.parse::<AccountId>().unwrap()).into();

    let params = Params {
        maker: MAKER.parse().unwrap(),
        src_token,
        dst_token,
        price: "1".parse().unwrap(),
        deadline: Timestamp::now() + Duration::from_mins(1),
        partial_fills_allowed: false,
        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        taker_whitelist: BTreeSet::default(),
        protocol_fees: None,
        integrator_fees: [
            (COLLECTOR1.parse::<AccountId>().unwrap(), Params::MAX_FEE),
            (
                COLLECTOR2.parse::<AccountId>().unwrap(),
                Pips::from_percent(1).unwrap(),
            ),
        ]
        .into(),
        auth_caller: None,
        salt: [0; 32],
    };

    let result = ContractStorage::init_state(&params);
    assert!(matches!(result, Err(Error::ExcessiveFees)));
}

#[test]
fn test_combined_protocol_and_integrator_fees_does_not_pass_validation() {
    let src_token: TokenId = Nep141TokenId::new(SRC_TOKEN.parse::<AccountId>().unwrap()).into();
    let dst_token: TokenId = Nep141TokenId::new(DST_TOKEN.parse::<AccountId>().unwrap()).into();

    let params = Params {
        maker: MAKER.parse().unwrap(),
        src_token,
        dst_token,
        price: "1".parse().unwrap(),
        deadline: Timestamp::now() + Duration::from_mins(1),
        partial_fills_allowed: false,
        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        taker_whitelist: BTreeSet::default(),
        protocol_fees: Some(ProtocolFees {
            fee: Params::MAX_FEE,
            surplus: Pips::ZERO,
            collector: COLLECTOR1.parse().unwrap(),
        }),
        integrator_fees: [(
            COLLECTOR2.parse::<AccountId>().unwrap(),
            Pips::from_percent(1).unwrap(),
        )]
        .into(),
        auth_caller: None,
        salt: [0; 32],
    };

    let result = ContractStorage::init_state(&params);
    assert!(matches!(result, Err(Error::ExcessiveFees)));
}

#[test]
fn test_valid_combined_fees_within_cap() {
    let src_token: TokenId = Nep141TokenId::new(SRC_TOKEN.parse::<AccountId>().unwrap()).into();
    let dst_token: TokenId = Nep141TokenId::new(DST_TOKEN.parse::<AccountId>().unwrap()).into();

    let params = Params {
        maker: MAKER.parse().unwrap(),
        src_token,
        dst_token,
        price: "1".parse().unwrap(),
        deadline: Timestamp::now() + Duration::from_mins(1),
        partial_fills_allowed: false,
        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        taker_whitelist: BTreeSet::default(),
        protocol_fees: Some(ProtocolFees {
            fee: Params::MAX_FEE - Pips::ONE_PERCENT,
            surplus: Pips::ZERO,
            collector: COLLECTOR1.parse().unwrap(),
        }),
        integrator_fees: [(
            COLLECTOR2.parse::<AccountId>().unwrap(),
            Pips::from_percent(1).unwrap(),
        )]
        .into(),
        auth_caller: None,
        salt: [0; 32],
    };

    let result = ContractStorage::init_state(&params);
    assert!(result.is_ok());
}

#[rstest]
#[tokio::test]
async fn test_surplus_fee_is_uncapped(#[future(awt)] env: Env) {
    const MAKER_AMOUNT: u128 = 10_000;
    const TAKER_AMOUNT: u128 = 20_000;
    const EXPECTED_COLLECTOR_FEE: u128 = 10_200;

    let [src_verifier_asset, dst_verifier_asset] =
        [env.src_ft.contract_id(), env.dst_ft.contract_id()]
            .map(Clone::clone)
            .map(Nep141TokenId::new)
            .map(TokenId::from);

    let [src_token, dst_token] = [&src_verifier_asset, &dst_verifier_asset]
        .map(|token_id| {
            Nep245TokenId::new(env.verifier.contract_id().clone(), token_id.to_string())
        })
        .map(Into::<TokenId>::into);

    let params = Params {
        maker: env.maker.account_id().clone(),
        src_token: src_token.clone(),
        dst_token: dst_token.clone(),
        price: "1".parse().unwrap(),
        deadline: Timestamp::now() + Duration::from_mins(1),
        partial_fills_allowed: true,
        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        taker_whitelist: env.takers.iter().map(Near::account_id).cloned().collect(),
        protocol_fees: ProtocolFees {
            fee: Pips::from_percent(1).unwrap(),
            surplus: Pips::from_percent(100).unwrap(),
            collector: env.fee_collectors[0].account_id().clone(),
        }
        .into(),
        integrator_fees: BTreeMap::default(),
        auth_caller: Some(env.verifier.contract_id().clone()),
        salt: [0; 32],
    };

    let state_init = StateInit::V1(StateInitV1 {
        code: env.escrow_global_id.clone(),
        data: ContractStorage::init_state(&params).unwrap(),
    });

    let escrow_id = state_init.derive_account_id();

    let deposited = env
        .maker
        .ft(env.src_ft.contract_id())
        .unwrap()
        .transfer_call(
            env.verifier.contract_id(),
            MAKER_AMOUNT,
            serde_json::to_string(
                &DepositMessage::new(escrow_id.clone()).with_action(DepositAction::Notify(
                    NotifyOnTransfer::new(
                        serde_json::to_string(&TransferMessage {
                            params: params.clone(),
                            action: TransferAction::Fund,
                        })
                        .unwrap(),
                    )
                    .with_state_init(state_init.clone()),
                )),
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .json::<U128>()
        .unwrap();

    assert_eq!(deposited.0, MAKER_AMOUNT);

    let escrow_state = env.contract::<Escrow>(&escrow_id).es_view().await;
    assert!(escrow_state.is_ok());

    // Taker fills at price "2" — sends 20,000 dst tokens for 10,000 src
    // surplus = taker_dst_used - src_out * maker_price = 20,000 - 10,000 = 10,000
    let deposited_on_fill = env.takers[0]
        .ft(env.dst_ft.contract_id())
        .unwrap()
        .transfer_call(
            env.verifier.contract_id(),
            TAKER_AMOUNT,
            serde_json::to_string(
                &DepositMessage::new(escrow_id.clone()).with_action(DepositAction::Notify(
                    NotifyOnTransfer::new(
                        serde_json::to_string(&TransferMessage {
                            params: params.clone(),
                            action: FillAction {
                                price: "2".parse().unwrap(),
                                deadline: Timestamp::now() + Duration::from_secs(10),
                                receive_src_to: OverrideSend::default(),
                            }
                            .into(),
                        })
                        .unwrap(),
                    ),
                )),
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .json::<U128>()
        .unwrap();

    assert_eq!(deposited_on_fill.0, TAKER_AMOUNT);

    let collector_balance = env
        .contract::<Mt>(env.verifier.contract_id())
        .mt_balance_of(MtBalanceOfArgs {
            account_id: env.fee_collectors[0].account_id(),
            token_id: &dst_verifier_asset.to_string(),
        })
        .await
        .unwrap()
        .0;

    assert_eq!(collector_balance, EXPECTED_COLLECTOR_FEE);

    let max_capped_fee = Params::MAX_FEE.fee(TAKER_AMOUNT);
    assert!(collector_balance > max_capped_fee);
}
