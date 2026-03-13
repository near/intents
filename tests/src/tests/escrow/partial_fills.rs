use defuse_sandbox::extensions::escrow::contract::{
    ContractStorage, Deadline, OverrideSend, Params, Pips, ProtocolFees,
    action::{FillAction, TransferAction, TransferMessage},
    token_id::{TokenId, nep141::Nep141TokenId, nep245::Nep245TokenId},
};
use defuse_sandbox::extensions::{
    defuse::contract::{
        core::intents::tokens::NotifyOnTransfer,
        tokens::{DepositAction, DepositMessage},
    },
    escrow::{EscrowExt, EscrowExtView},
};
use defuse_sandbox::{
    Account, anyhow,
    extensions::{ft::FtExt, mt::MtViewExt},
};
use futures::{TryStreamExt, stream::FuturesOrdered};
use itertools::Itertools;
use near_sdk::{
    AccountIdRef, serde_json,
    state_init::{StateInit, StateInitV1},
};
use std::time::Duration;

use rstest::rstest;

use crate::tests::escrow::helpers::{Env, env};

#[rstest]
#[tokio::test]
async fn partial_fills(#[future(awt)] env: Env) {
    const MAKER_AMOUNT: u128 = 10000;
    // const TAKER_AMOUNT: u128 = 20000;
    const TIMEOUT: Duration = Duration::from_secs(60);

    // try_join!(
    //     env.src_deposit_to_verifier(env.maker.id(), SRC_TOKEN_ID, MAKER_AMOUNT),
    //     env.dst_deposit_to_verifier(env.takers[0].id(), DST_TOKEN_ID, TAKER_AMOUNT),
    //     env.dst_deposit_to_verifier(env.takers[1].id(), DST_TOKEN_ID, TAKER_AMOUNT),
    //     env.dst_deposit_to_verifier(env.takers[2].id(), DST_TOKEN_ID, TAKER_AMOUNT),
    // )
    // .unwrap();

    let [src_verifier_asset, dst_verifier_asset] = [env.src_ft.id(), env.dst_ft.id()]
        .map(Clone::clone)
        .map(Nep141TokenId::new)
        .map(TokenId::from);

    let [src_token, dst_token] = [&src_verifier_asset, &dst_verifier_asset]
        .map(|token_id| Nep245TokenId::new(env.verifier.id().clone(), token_id.to_string()))
        .map(Into::<TokenId>::into);

    let params = Params {
        maker: env.maker.id().clone(),

        src_token: src_token.clone(),
        dst_token: dst_token.clone(),

        price: "2".parse().unwrap(),
        deadline: Deadline::timeout(TIMEOUT),

        partial_fills_allowed: true,

        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        // taker_whitelist: Default::default(),
        taker_whitelist: env.takers.iter().map(|a| a.id()).cloned().collect(),
        protocol_fees: ProtocolFees {
            fee: Pips::from_percent(1).unwrap(),
            surplus: Pips::from_percent(10).unwrap(),
            collector: env.fee_collectors[0].id().clone(),
        }
        .into(),
        integrator_fees: env
            .fee_collectors
            .iter()
            .map(|a| a.id())
            .cloned()
            .enumerate()
            .map(|(percent, a)| {
                (
                    a,
                    Pips::from_percent(u32::try_from(percent).unwrap() + 1).unwrap(),
                )
            })
            .skip(1)
            .collect(),
        auth_caller: Some(env.verifier.id().clone()),
        salt: [0; 32],
    };
    let state_init = StateInit::V1(StateInitV1 {
        code: env.escrow_global_id.clone(),
        data: ContractStorage::init_state(&params).unwrap(),
    });

    let escrow = env.account(state_init.derive_account_id());

    show_verifier_balances(
        &env.verifier,
        [escrow.id(), env.maker.id()]
            .into_iter()
            .chain(env.takers.iter().map(|a| a.id()))
            .chain(env.fee_collectors.iter().map(|a| a.id()))
            .map(AsRef::as_ref),
        &[&src_verifier_asset, &dst_verifier_asset],
    )
    .await;

    // maker deposit
    {
        for amount in [MAKER_AMOUNT - 100, 100] {
            let deposited = env
                .maker
                .ft_transfer_call(
                    env.src_ft.id(),
                    env.verifier.id(),
                    amount,
                    None,
                    serde_json::to_string(
                        &DepositMessage::new(escrow.id().clone()).with_action(
                            DepositAction::Notify(
                                NotifyOnTransfer::new(
                                    serde_json::to_string(&TransferMessage {
                                        params: params.clone(),
                                        action: TransferAction::Fund,
                                    })
                                    .unwrap(),
                                )
                                .with_state_init(state_init.clone()),
                            ),
                        ),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();

            println!("maker sent: {amount}, deposited: {deposited}");

            show_verifier_balances(
                &env.verifier,
                [escrow.id(), env.maker.id()]
                    .into_iter()
                    .chain(env.takers.iter().map(|a| a.id()))
                    .chain(env.fee_collectors.iter().map(|a| a.id()))
                    .map(AsRef::as_ref),
                &[&src_verifier_asset, &dst_verifier_asset],
            )
            .await;

            assert_eq!(deposited, amount);
            maybe_view_escrow(&escrow).await;
        }
    }

    // takers deposit
    {
        for (taker, amount) in env.takers.iter().zip([10000, 5000, 20000]) {
            let deposited = taker
                .ft_transfer_call(
                    env.dst_ft.id(),
                    env.verifier.id(),
                    amount,
                    None,
                    serde_json::to_string(
                        &DepositMessage::new(escrow.id().clone()).with_action(
                            DepositAction::Notify(NotifyOnTransfer::new(
                                serde_json::to_string(&TransferMessage {
                                    params: params.clone(),
                                    action: FillAction {
                                        price: "2.1".parse().unwrap(),
                                        deadline: Deadline::timeout(Duration::from_secs(10)),
                                        receive_src_to: OverrideSend {
                                            memo: Some("taker memo".to_string()),
                                            // msg: Some("taker msg".to_string()),
                                            ..Default::default()
                                        },
                                    }
                                    .into(),
                                })
                                .unwrap(),
                            )),
                        ),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();

            println!("taker sent: {amount}, deposited: {deposited}");

            show_verifier_balances(
                &env.verifier,
                [escrow.id(), env.maker.id()]
                    .into_iter()
                    .chain(env.takers.iter().map(|a| a.id()))
                    .chain(env.fee_collectors.iter().map(|a| a.id()))
                    .map(AsRef::as_ref),
                &[&src_verifier_asset, &dst_verifier_asset],
            )
            .await;

            // assert_eq!(sent, amount);
        }
        maybe_view_escrow(&escrow).await;
    }

    // TODO: fast-forward
    // tokio::time::sleep(TIMEOUT).await;

    // maker closes the escrow
    {
        env.maker.es_close(escrow.id(), &params).await.unwrap();

        show_verifier_balances(
            &env.verifier,
            [escrow.id(), env.maker.id()]
                .into_iter()
                .chain(env.takers.iter().map(|a| a.id()))
                .chain(env.fee_collectors.iter().map(|a| a.id()))
                .map(AsRef::as_ref),
            &[&src_verifier_asset, &dst_verifier_asset],
        )
        .await;
        maybe_view_escrow(&escrow).await;

        // escrow
        //     .view()
        //     .await
        //     .expect_err("cleanup should have been performed");
    }
}

pub async fn show_verifier_balances(
    verifier: &Account,
    accounts: impl IntoIterator<Item = &AccountIdRef>,
    token_ids: &[&TokenId],
) {
    let mut balances = accounts
        .into_iter()
        .map(|account_id| async move {
            let balances = verifier
                .mt_batch_balance_of(account_id, token_ids.iter().map(ToString::to_string))
                .await?;
            anyhow::Ok((account_id, balances))
        })
        .collect::<FuturesOrdered<_>>();

    while let Some((account_id, balances)) = balances.try_next().await.unwrap() {
        println!(
            "{:<64} {}",
            account_id,
            balances.into_iter().map(|b| format!("{b:<30}")).join(" ")
        );
    }
}

async fn maybe_view_escrow(escrow: &Account) {
    let Ok(account) = escrow.view().await else {
        println!("{} does not exist", escrow.id());
        return;
    };
    println!("{}: {:?}", escrow.id(), account);
    let s = escrow.es_view().await.unwrap();
    println!(
        "{}::es_view() -> {:#}",
        escrow.id(),
        serde_json::to_value(&s).unwrap()
    );
}
