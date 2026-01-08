use std::time::Duration;

use defuse::{
    core::intents::tokens::NotifyOnTransfer,
    tokens::{DepositAction, DepositMessage},
};
use defuse_escrow_swap::{
    ContractStorage, Deadline, OverrideSend, Params, Pips, ProtocolFees,
    action::{FillAction, TransferAction, TransferMessage},
    token_id::{TokenId, nep141::Nep141TokenId, nep245::Nep245TokenId},
};
use defuse_sandbox::{
    Account,
    extensions::{ft::FtExt, mt::MtViewExt},
};
use futures::{TryStreamExt, stream::FuturesOrdered};
use itertools::Itertools;
use near_sdk::{
    AccountIdRef,
    state_init::{StateInit, StateInitV1},
};
use rstest::rstest;

use crate::tests::escrow::{
    EscrowExt, EscrowExtView,
    env::{Env, env},
};

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

/// Test partial fill with dust: verifies remaining funds returned to maker after timeout
#[tokio::test]
async fn test_partial_fill_funds_returned_after_timeout() {
    use super::EscrowExt;
    use crate::tests::defuse::env::Env as DefuseEnv;
    use defuse_escrow_swap::ParamsBuilder;
    use defuse_escrow_swap::action::{FillMessageBuilder, FundMessageBuilder};
    use defuse_escrow_swap::decimal::UD128;
    use defuse_sandbox::{MtExt, MtViewExt};
    use defuse_sandbox_ext::EscrowSwapAccountExt;

    let env = DefuseEnv::builder().build().await;
    let escrow_swap_global = env.root().deploy_escrow_swap_global("escrow_swap").await;

    let maker = env.create_named_user("maker").await;
    let taker = env.create_named_user("taker").await;

    // Price 0.333333: taker pays 0.333333 dst per 1 src (gets ~3 src per 1 dst)
    // Maker deposits 1000 src, taker fills with 166 dst (~50%)
    // floor(166 / 0.333333) = 498 src to taker
    // 1000 - 498 = 502 src remaining (partial + rounding dust)
    let maker_balance = 1_000_u128;
    let fill_amount = 166_u128;
    let price: UD128 = "0.333333".parse().unwrap();
    let expected_taker_src = 498_u128; // floor(166 / 0.333333)
    let expected_maker_refund = maker_balance - expected_taker_src; // 502

    let ((_, src_token_id), (_, dst_token_id)) = futures::try_join!(
        env.create_mt_token_with_initial_balances([(maker.id().clone(), maker_balance)]),
        env.create_mt_token_with_initial_balances([(taker.id().clone(), fill_amount)]),
    )
    .unwrap();

    let src_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        src_token_id.to_string(),
    ));
    let dst_token = TokenId::from(Nep245TokenId::new(
        env.defuse.id().clone(),
        dst_token_id.to_string(),
    ));

    let escrow_params = ParamsBuilder::new(
        (maker.id().clone(), src_token),
        ([taker.id().clone()], dst_token),
    )
    .with_price(price)
    .with_partial_fills_allowed(true)
    .with_deadline(Deadline::timeout(Duration::from_secs(6)))
    .build();
    let fund_msg = FundMessageBuilder::new(escrow_params.clone()).build();
    let fill_msg = FillMessageBuilder::new(escrow_params.clone())
        .with_deadline(Deadline::timeout(Duration::from_secs(5)))
        .build();

    let escrow_id = env
        .root()
        .deploy_escrow_swap_instance(escrow_swap_global, &escrow_params)
        .await;

    // Fund escrow
    maker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &src_token_id.to_string(),
            maker_balance,
            None,
            &serde_json::to_string(&fund_msg).unwrap(),
        )
        .await
        .unwrap();

    // Partial fill (~50%)
    taker
        .mt_transfer_call(
            env.defuse.id(),
            &escrow_id,
            &dst_token_id.to_string(),
            fill_amount,
            None,
            &serde_json::to_string(&fill_msg).unwrap(),
        )
        .await
        .unwrap();

    // Verify taker received expected src
    let taker_src = env
        .defuse
        .mt_balance_of(taker.id(), &src_token_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        taker_src, expected_taker_src,
        "taker should have floor(166/0.333333) = 498"
    );

    // Maker has 0 before close (remaining in escrow)
    let maker_src_before = env
        .defuse
        .mt_balance_of(maker.id(), &src_token_id.to_string())
        .await
        .unwrap();
    assert_eq!(maker_src_before, 0, "maker src should be 0 before close");

    // Wait for deadline to expire
    tokio::time::sleep(Duration::from_secs(7)).await;

    // Close escrow - remaining funds return to maker
    maker.es_close(&escrow_id, &escrow_params).await.unwrap();

    // Verify maker received remaining src (partial fill remainder + rounding dust)
    let maker_src_after = env
        .defuse
        .mt_balance_of(maker.id(), &src_token_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        maker_src_after, expected_maker_refund,
        "maker should receive 502 src after close"
    );
}
