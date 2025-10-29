mod env;

use std::time::Duration;

use chrono::Utc;
use defuse_escrow::{Action, FillAction, FixedParams, OpenAction, Params, Price, TransferMessage};
use defuse_fees::Pips;
use defuse_token_id::{TokenId, nep245::Nep245TokenId};
use futures::{StreamExt, join, stream::FuturesOrdered};
use near_sdk::{
    AccountId, NearToken,
    json_types::U128,
    serde_json::{self, json},
};

use crate::env::Env;

#[tokio::test]
async fn escrow() {
    const MAKER_AMOUNT: u128 = 100;
    const TAKER_AMOUNT: u128 = 200;

    let env = Env::new().await;

    let long = |name: &str| {
        format!(
            "{name}-{}",
            "0".repeat(AccountId::MAX_LEN - env.root_id().len() - 1 - name.len() - 1)
        )
    };

    let (
        maker,
        taker1,
        taker2,
        taker3,
        mt_src,
        mt_dst,
        fee_collector1,
        fee_collector2,
        fee_collector3,
        cancel_authorify,
    ) = join!(
        env.create_subaccount(long("maker"), NearToken::from_near(10)),
        env.create_subaccount(long("taker1"), NearToken::from_near(10)),
        env.create_subaccount(long("taker2"), NearToken::from_near(10)),
        env.create_subaccount(long("taker3"), NearToken::from_near(10)),
        env.create_subaccount(long("mt-src"), NearToken::from_near(10)),
        env.create_subaccount(long("mt-dst"), NearToken::from_near(10)),
        env.create_subaccount(long("fee-collector1"), NearToken::from_near(10)),
        env.create_subaccount(long("fee-collector2"), NearToken::from_near(10)),
        env.create_subaccount(long("fee-collector3"), NearToken::from_near(10)),
        env.create_subaccount(long("cancel"), NearToken::from_near(10)),
    );

    let [src_asset, dst_asset]: [Nep245TokenId; 2] =
        [(mt_src.0.clone(), "1"), (mt_dst.0.clone(), "2")].map(|(mt, token_id)| {
            Nep245TokenId::new(
                mt,
                token_id.repeat(15), // TODO: 55
            )
            .unwrap()
        });

    {
        let env = &env;
        let deposit = |mt, signer, receiver_id, token_id, amount| async move {
            env.verifier
                .call_function(
                    "mt_on_transfer",
                    json!({
                        "sender_id": mt,
                        "previous_owner_ids": [mt],
                        "token_ids": [token_id],
                        "amounts": [U128(amount)],
                        "msg": receiver_id,
                    }),
                )
                .unwrap()
                .transaction()
                .with_signer(mt, signer)
                .send_to(env.network_config())
                .await
                .unwrap()
                .into_result()
                .unwrap()
        };

        join!(
            deposit(
                mt_src.0.clone(),
                mt_src.1.clone(),
                maker.0.clone(),
                src_asset.mt_token_id(),
                MAKER_AMOUNT
            ),
            deposit(
                mt_dst.0.clone(),
                mt_dst.1.clone(),
                taker1.0.clone(),
                dst_asset.mt_token_id(),
                TAKER_AMOUNT
            ),
            deposit(
                mt_dst.0.clone(),
                mt_dst.1.clone(),
                taker2.0.clone(),
                dst_asset.mt_token_id(),
                TAKER_AMOUNT
            ),
            deposit(
                mt_dst.0.clone(),
                mt_dst.1.clone(),
                taker3.0.clone(),
                dst_asset.mt_token_id(),
                TAKER_AMOUNT
            ),
        );
    }

    let [src_asset, dst_asset] = [src_asset, dst_asset].map(|t| TokenId::Nep245(t));

    let [src_asset, dst_asset] = [src_asset.clone(), dst_asset.clone()]
        .map(|token_id| Nep245TokenId::new(env.verifier.0.clone(), token_id.to_string()).unwrap());

    let fixed_params = FixedParams {
        maker: maker.0.clone(),
        src_asset: src_asset.clone(),
        dst_asset: dst_asset.clone(),
        maker_dst_receiver_id: Some(maker.0.clone()),
        partial_fills_allowed: true,
        fees: [&fee_collector1.0, &fee_collector2.0, &fee_collector3.0]
            .into_iter()
            .cloned()
            .enumerate()
            .map(|(percent, a)| (a, Pips::from_percent(percent as u32 + 1).unwrap()))
            .collect(),
        taker_whitelist: [taker1.0.clone(), taker2.0.clone(), taker3.0.clone()].into(),
        maker_authority: Some(cancel_authorify.0.clone()),
    };

    let escrow = env
        .create_escrow(
            &fixed_params,
            Params {
                price: Price::ratio(MAKER_AMOUNT, TAKER_AMOUNT).unwrap(),
                deadline: Utc::now() + Duration::from_secs(120),
            },
        )
        .await;
    println!("escrow id: {}", escrow.0);

    let show_balances = || async {
        let mut balances = [
            &escrow.0,
            &maker.0,
            &taker1.0,
            &taker2.0,
            &taker3.0,
            &mt_src.0,
            &mt_dst.0,
            &fee_collector1.0,
            &fee_collector2.0,
            &fee_collector3.0,
            &cancel_authorify.0,
        ]
        .into_iter()
        .map(|account_id| {
            let (src_asset, dst_asset, env) = (&src_asset, &dst_asset, &env);
            async move {
                let balances: Vec<U128> = env
                    .verifier
                    .call_function(
                        "mt_batch_balance_of",
                        json!({
                            "account_id": account_id,
                            "token_ids": [src_asset.mt_token_id(), dst_asset.mt_token_id()],
                        }),
                    )
                    .unwrap()
                    .read_only()
                    .fetch_from(&env.network_config())
                    .await
                    .unwrap()
                    .data;
                (
                    account_id,
                    <[U128; 2]>::try_from(balances).unwrap().map(|a| a.0),
                )
            }
        })
        .collect::<FuturesOrdered<_>>();

        println!("balances:");
        while let Some((account_id, [src_balance, dst_balance])) = balances.next().await {
            println!("{account_id}\tsrc: {src_balance}\tdst: {dst_balance}");
        }
    };

    let view = || async {
        let view = escrow
            .call_function("view", ())
            .unwrap()
            .read_only::<serde_json::Value>()
            .fetch_from(env.network_config())
            .await
            .unwrap()
            .data;
        println!("view: {}", view);
    };

    view().await;
    show_balances().await;

    // maker deposit
    {
        let deposited = env
            .verifier
            .call_function(
                "mt_transfer_call",
                json!({
                    "receiver_id": escrow.0.clone(),
                    "token_id": src_asset.mt_token_id(),
                    "amount": U128(MAKER_AMOUNT),
                    "msg": serde_json::to_string(&TransferMessage {
                        fixed_params: fixed_params.clone(),
                        action: Action::Open(OpenAction { new_price: None }),
                    }).unwrap(),
                }),
            )
            .unwrap()
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .with_signer(maker.0.clone(), maker.1.clone())
            .send_to(env.network_config())
            .await
            .unwrap()
            .into_result()
            .unwrap()
            .json::<Vec<U128>>()
            .unwrap()[0]
            .0;

        println!("maker deposited: {deposited}");
        show_balances().await;

        assert_eq!(deposited, MAKER_AMOUNT);
    }

    view().await;

    // taker deposit
    {
        let deposited = env
            .verifier
            .call_function(
                "mt_transfer_call",
                json!({
                    "receiver_id": escrow.0.clone(),
                    "token_id": dst_asset.mt_token_id(),
                    "amount": U128(TAKER_AMOUNT),
                    "msg": serde_json::to_string(&TransferMessage {
                        fixed_params,
                        action: Action::Fill(FillAction { receiver_id: None }),
                    }).unwrap(),
                }),
            )
            .unwrap()
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .with_signer(taker1.0.clone(), taker1.1.clone())
            .send_to(env.network_config())
            .await
            .unwrap()
            .into_result()
            .unwrap()
            .json::<Vec<U128>>()
            .unwrap()[0]
            .0;

        println!("taker deposited: {deposited}");
        show_balances().await;

        assert_eq!(deposited, TAKER_AMOUNT);
    }

    view().await;
}
