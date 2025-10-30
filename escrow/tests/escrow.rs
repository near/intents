mod env;

use std::time::Duration;

use chrono::Utc;
use defuse_escrow::{Action, FillAction, FixedParams, OpenAction, Params, Price, TransferMessage};
use defuse_fees::Pips;
use defuse_token_id::{TokenId, nep245::Nep245TokenId};
use futures::{StreamExt, join, stream::FuturesOrdered};
use impl_tools::autoimpl;
use itertools::Itertools;
use near_sdk::{
    AccountId, AccountIdRef, Gas, NearToken,
    json_types::U128,
    serde_json::{self, json},
};

use crate::env::{Account, Env, SigningAccount};

#[autoimpl(Deref using self.env)]
struct TestEnv {
    env: Env,

    src_mt: SigningAccount,
    mt_dst: SigningAccount,

    maker: SigningAccount,
    takers: [SigningAccount; 3],

    fee_collectors: [SigningAccount; 3],
}

impl TestEnv {
    pub async fn new() -> Self {
        let env = Env::new().await;

        let long = |name: &str| {
            format!(
                "{name}-{}",
                "0".repeat(AccountId::MAX_LEN - env.id().len() - 1 - name.len() - 1)
            )
        };

        let (
            mt_src,
            mt_dst,
            maker,
            taker1,
            taker2,
            taker3,
            fee_collector1,
            fee_collector2,
            fee_collector3,
        ) = join!(
            env.create_subaccount(long("mt-src"), NearToken::from_near(10)),
            env.create_subaccount(long("mt-dst"), NearToken::from_near(10)),
            env.create_subaccount(long("maker"), NearToken::from_near(10)),
            env.create_subaccount(long("taker1"), NearToken::from_near(10)),
            env.create_subaccount(long("taker2"), NearToken::from_near(10)),
            env.create_subaccount(long("taker3"), NearToken::from_near(10)),
            env.create_subaccount(long("fee-collector1"), NearToken::from_near(10)),
            env.create_subaccount(long("fee-collector2"), NearToken::from_near(10)),
            env.create_subaccount(long("fee-collector3"), NearToken::from_near(10)),
        );

        Self {
            env,
            src_mt: mt_src,
            mt_dst,
            maker,
            takers: [taker1, taker2, taker3],
            fee_collectors: [fee_collector1, fee_collector2, fee_collector3],
        }
    }

    pub async fn src_deposit_to_verifier(
        &self,
        receiver_id: &AccountIdRef,
        token_id: &str,
        amount: u128,
    ) -> u128 {
        self.deposit_to_verifier(&self.src_mt, receiver_id, token_id, amount)
            .await
    }

    pub async fn dst_deposit_to_verifier(
        &self,
        receiver_id: &AccountIdRef,
        token_id: &str,
        amount: u128,
    ) -> u128 {
        self.deposit_to_verifier(&self.mt_dst, receiver_id, token_id, amount)
            .await
    }

    async fn deposit_to_verifier(
        &self,
        mt: &SigningAccount,
        receiver_id: &AccountIdRef,
        token_id: &str,
        amount: u128,
    ) -> u128 {
        let [refund] = mt
            .tx(self.verifier.id().clone())
            .function_call_json(
                "mt_on_transfer",
                json!({
                    "sender_id": self.src_mt.id(),
                    "previous_owner_ids": [mt.id()],
                    "token_ids": [token_id],
                    "amounts": [U128(amount)],
                    "msg": receiver_id,
                }),
                Gas::from_tgas(30),
                NearToken::from_yoctonear(0),
            )
            .await
            .unwrap()
            .into_result()
            .unwrap()
            .json::<Vec<U128>>()
            .expect("JSON")
            .try_into()
            .expect("more than one token refunded");
        amount - refund.0
    }

    pub async fn show_verifier_balances(
        &self,
        accounts: impl IntoIterator<Item = &AccountIdRef>,
        token_ids: &[&TokenId],
    ) {
        let mut balances = accounts
            .into_iter()
            .map(|account_id| async move {
                let balances = self
                    .verifier
                    .call_function_json::<Vec<U128>>(
                        "mt_batch_balance_of",
                        json!({
                            "account_id": account_id,
                            "token_ids": token_ids,
                        }),
                    )
                    .await;
                (account_id, balances)
            })
            .collect::<FuturesOrdered<_>>();

        while let Some((account_id, balances)) = balances.next().await {
            println!(
                "{:<64} {}",
                account_id,
                balances
                    .into_iter()
                    .map(|b| format!("{:<30}", b.0))
                    .join(" ")
            );
        }
    }

    pub async fn view_escrow(&self, escrow: &Account) {
        let s = escrow
            .call_function_json::<serde_json::Value>("view", ())
            .await;
        println!("{}::view() -> {:#}", escrow.id(), s);
    }
}

#[tokio::test]
async fn escrow() {
    const SRC_TOKEN_ID: &str = "src";
    const DST_TOKEN_ID: &str = "dst";

    const MAKER_AMOUNT: u128 = 100;
    const TAKER_AMOUNT: u128 = 200;

    let env = TestEnv::new().await;

    join!(
        env.src_deposit_to_verifier(env.maker.id(), SRC_TOKEN_ID, MAKER_AMOUNT),
        env.dst_deposit_to_verifier(env.takers[0].id(), DST_TOKEN_ID, TAKER_AMOUNT),
        env.dst_deposit_to_verifier(env.takers[1].id(), DST_TOKEN_ID, TAKER_AMOUNT),
        env.dst_deposit_to_verifier(env.takers[2].id(), DST_TOKEN_ID, TAKER_AMOUNT),
    );

    let [src_verifier_asset, dst_verifier_asset] = [
        (env.src_mt.id(), SRC_TOKEN_ID),
        (env.mt_dst.id(), DST_TOKEN_ID),
    ]
    .map(|(contract_id, token_id)| {
        Nep245TokenId::new(contract_id.clone(), token_id.to_string()).unwrap()
    })
    .map(TokenId::from);

    let [src_asset, dst_asset] = [&src_verifier_asset, &dst_verifier_asset].map(|token_id| {
        Nep245TokenId::new(env.verifier.id().clone(), token_id.to_string()).unwrap()
    });

    let fixed_params = FixedParams {
        maker: env.maker.id().clone(),
        refund_to: Some(env.maker.id().clone()),
        src_asset: src_asset.clone(),
        dst_asset: dst_asset.clone(),
        maker_dst_receiver_id: Some(env.maker.id().clone()),
        partial_fills_allowed: true,
        fees: env
            .fee_collectors
            .iter()
            .map(|a| a.id())
            .cloned()
            .enumerate()
            .map(|(percent, a)| (a, Pips::from_percent(percent as u32 + 1).unwrap()))
            .collect(),
        taker_whitelist: env.takers.iter().map(|a| a.id()).cloned().collect(),
        // maker_authority: Some(cancel_authorify.0.clone()),
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
    env.view_escrow(&escrow).await;
    env.show_verifier_balances(
        [escrow.id(), env.maker.id()]
            .into_iter()
            .chain(env.takers.iter().map(|a| a.id()))
            .chain(env.fee_collectors.iter().map(|a| a.id()))
            .map(|a| a.as_ref()),
        &[&src_verifier_asset, &dst_verifier_asset],
    )
    .await;

    // maker deposit
    {
        let sent = env
            .maker
            .mt_transfer_call(
                env.verifier.id(),
                escrow.id(),
                src_verifier_asset.to_string(),
                MAKER_AMOUNT,
                serde_json::to_string(&TransferMessage {
                    fixed_params: fixed_params.clone(),
                    action: Action::Open(OpenAction { new_price: None }),
                })
                .unwrap(),
            )
            .await;

        println!("maker deposited: {sent}");

        env.show_verifier_balances(
            [escrow.id(), env.maker.id()]
                .into_iter()
                .chain(env.takers.iter().map(|a| a.id()))
                .chain(env.fee_collectors.iter().map(|a| a.id()))
                .map(|a| a.as_ref()),
            &[&src_verifier_asset, &dst_verifier_asset],
        )
        .await;

        assert_eq!(sent, MAKER_AMOUNT);
    }

    env.view_escrow(&escrow).await;

    // taker deposit
    {
        for (taker, amount) in env.takers.iter().zip([100, 50, 30]) {
            let sent = taker
                .mt_transfer_call(
                    env.verifier.id(),
                    escrow.id(),
                    dst_verifier_asset.to_string(),
                    amount,
                    serde_json::to_string(&TransferMessage {
                        fixed_params: fixed_params.clone(),
                        action: Action::Fill(FillAction { receiver_id: None }),
                    })
                    .unwrap(),
                )
                .await;

            println!("taker deposited: {sent}");

            env.show_verifier_balances(
                [escrow.id(), env.maker.id()]
                    .into_iter()
                    .chain(env.takers.iter().map(|a| a.id()))
                    .chain(env.fee_collectors.iter().map(|a| a.id()))
                    .map(|a| a.as_ref()),
                &[&src_verifier_asset, &dst_verifier_asset],
            )
            .await;

            assert_eq!(sent, amount);
        }
    }

    env.view_escrow(&escrow).await;
}
