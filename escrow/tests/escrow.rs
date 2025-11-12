mod env;

use std::time::Duration;

use defuse_escrow::{
    Deadline, FillAction, FixedParams, OpenAction, OverrideSend, Params, Price, TransferMessage,
};
use defuse_fees::Pips;
use defuse_sandbox::{
    Account, MtExt, MtViewExt, SigningAccount, TxResult,
    api::types::errors::{DataConversionError, ExecutionError},
};
use defuse_token_id::{TokenId, nep245::Nep245TokenId};
use futures::{TryStreamExt, stream::FuturesOrdered, try_join};
use impl_tools::autoimpl;
use itertools::Itertools;
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json};

use crate::env::{BaseEnv, EscrowExt, EscrowViewExt};

#[tokio::test]
async fn partial_fills() {
    const SRC_TOKEN_ID: &str = "src";
    const DST_TOKEN_ID: &str = "dst";

    const MAKER_AMOUNT: u128 = 10000;
    const TAKER_AMOUNT: u128 = 20000;

    let env = EscrowEnv::new().await.unwrap();

    try_join!(
        env.src_deposit_to_verifier(env.maker.id(), SRC_TOKEN_ID, MAKER_AMOUNT),
        env.dst_deposit_to_verifier(env.takers[0].id(), DST_TOKEN_ID, TAKER_AMOUNT),
        env.dst_deposit_to_verifier(env.takers[1].id(), DST_TOKEN_ID, TAKER_AMOUNT),
        env.dst_deposit_to_verifier(env.takers[2].id(), DST_TOKEN_ID, TAKER_AMOUNT),
    )
    .unwrap();

    let [src_verifier_asset, dst_verifier_asset] = [
        (env.src_mt.id(), SRC_TOKEN_ID),
        (env.mt_dst.id(), DST_TOKEN_ID),
    ]
    .map(|(contract_id, token_id)| {
        Nep245TokenId::new(contract_id.clone(), token_id.to_string()).unwrap()
    })
    .map(TokenId::from);

    let [src_asset, dst_asset] = [&src_verifier_asset, &dst_verifier_asset]
        .map(|token_id| {
            Nep245TokenId::new(env.verifier.id().clone(), token_id.to_string()).unwrap()
        })
        .map(Into::<TokenId>::into);

    let fixed_params = FixedParams {
        maker: env.maker.id().clone(),
        // refund_src_to: SendParams {
        //     receiver_id: None,
        //     memo: None,
        //     msg: Some("fail".to_string()),
        //     min_gas: None,
        // },
        refund_src_to: OverrideSend::default(),
        src_token: src_asset.clone(),
        dst_token: dst_asset.clone(),
        receive_dst_to: OverrideSend::default(),
        // receive_dst_to: SendParams {
        //     receiver_id: None,
        //     memo: None,
        //     msg: Some("fail".to_string()),
        //     min_gas: None,
        // },
        partial_fills_allowed: true,
        fees: env
            .fee_collectors
            .iter()
            .map(|a| a.id())
            .cloned()
            .enumerate()
            .map(|(percent, a)| (a, Pips::from_percent(percent as u32 + 1).unwrap()))
            .collect(),
        // taker_whitelist: Default::default(),
        taker_whitelist: env.takers.iter().map(|a| a.id()).cloned().collect(),
        // maker_authority: Some(cancel_authorify.0.clone()),
    };

    const TIMEOUT: Duration = Duration::from_secs(60);

    let escrow = env
        .create_escrow(
            &fixed_params,
            Params {
                price: Price::ratio(MAKER_AMOUNT, TAKER_AMOUNT).unwrap(),
                deadline: Deadline::timeout(TIMEOUT),
            },
        )
        .await
        .unwrap();
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
        for amount in [MAKER_AMOUNT - 100, 100] {
            let sent = env
                .maker
                .mt_transfer_call(
                    env.verifier.id().clone(),
                    escrow.id(),
                    src_verifier_asset.to_string(),
                    amount,
                    "maker deposit".to_string(),
                    serde_json::to_string(&TransferMessage {
                        fixed_params: fixed_params.clone(),
                        action: OpenAction { new_price: None }.into(),
                    })
                    .unwrap(),
                )
                .await
                .unwrap();

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

            assert_eq!(sent, amount);
            env.view_escrow(&escrow).await;
        }
    }

    // takers deposit
    {
        for (taker, amount) in env.takers.iter().zip([10000, 5000, 20000]) {
            let sent = taker
                .mt_transfer_call(
                    env.verifier.id().clone(),
                    escrow.id(),
                    dst_verifier_asset.to_string(),
                    amount,
                    "taker fill".to_string(),
                    serde_json::to_string(&TransferMessage {
                        fixed_params: fixed_params.clone(),
                        action: FillAction {
                            receive_src_to: OverrideSend {
                                memo: Some("taker memo".to_string()),
                                // msg: Some("taker msg".to_string()),
                                ..Default::default()
                            },
                        }
                        .into(),
                    })
                    .unwrap(),
                )
                .await
                .unwrap();

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

            // assert_eq!(sent, amount);
        }
        env.view_escrow(&escrow).await;
    }

    // TODO: fast-forward
    // tokio::time::sleep(TIMEOUT).await;

    // maker closes the escrow
    {
        env.maker
            .close_escrow(escrow.id().clone(), fixed_params.clone())
            .await
            .unwrap();

        env.show_verifier_balances(
            [escrow.id(), env.maker.id()]
                .into_iter()
                .chain(env.takers.iter().map(|a| a.id()))
                .chain(env.fee_collectors.iter().map(|a| a.id()))
                .map(|a| a.as_ref()),
            &[&src_verifier_asset, &dst_verifier_asset],
        )
        .await;

        escrow
            .view()
            .await
            .expect_err("cleanup should have been performed");
    }
}

#[autoimpl(Deref using self.env)]
struct EscrowEnv {
    env: BaseEnv,

    src_mt: SigningAccount,
    mt_dst: SigningAccount,

    maker: SigningAccount,
    takers: [SigningAccount; 3],

    fee_collectors: [SigningAccount; 3],
}

impl EscrowEnv {
    pub async fn new() -> TxResult<Self> {
        let env = BaseEnv::new().await?;
        let root = env.root();

        let long = |name: &str| {
            format!(
                "{name}-{}",
                "0".repeat(AccountId::MAX_LEN - root.id().len() - 1 - name.len() - 1)
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
        ) = try_join!(
            root.create_subaccount(long("mt-src"), NearToken::from_near(10)),
            root.create_subaccount(long("mt-dst"), NearToken::from_near(10)),
            root.create_subaccount(long("maker"), NearToken::from_near(10)),
            root.create_subaccount(long("taker1"), NearToken::from_near(10)),
            root.create_subaccount(long("taker2"), NearToken::from_near(10)),
            root.create_subaccount(long("taker3"), NearToken::from_near(10)),
            root.create_subaccount(long("fee-collector1"), NearToken::from_near(10)),
            root.create_subaccount(long("fee-collector2"), NearToken::from_near(10)),
            root.create_subaccount(long("fee-collector3"), NearToken::from_near(10)),
        )?;

        Ok(Self {
            env,
            src_mt: mt_src,
            mt_dst,
            maker,
            takers: [taker1, taker2, taker3],
            fee_collectors: [fee_collector1, fee_collector2, fee_collector3],
        })
    }

    pub async fn src_deposit_to_verifier(
        &self,
        receiver_id: &AccountIdRef,
        token_id: &str,
        amount: u128,
    ) -> TxResult<u128> {
        self.deposit_to_verifier(&self.src_mt, receiver_id, token_id, amount)
            .await
    }

    pub async fn dst_deposit_to_verifier(
        &self,
        receiver_id: &AccountIdRef,
        token_id: &str,
        amount: u128,
    ) -> TxResult<u128> {
        self.deposit_to_verifier(&self.mt_dst, receiver_id, token_id, amount)
            .await
    }

    async fn deposit_to_verifier(
        &self,
        mt: &SigningAccount,
        receiver_id: &AccountIdRef,
        token_id: &str,
        amount: u128,
    ) -> TxResult<u128> {
        mt.mt_on_transfer(
            mt.id(),
            self.verifier.id().clone(),
            [(token_id.to_string(), amount)],
            receiver_id.to_string(),
        )
        .await
        .and_then(|refunds| {
            let [refund] = refunds
                .try_into()
                .map_err(|refunds: Vec<_>| DataConversionError::IncorrectLength(refunds.len()))
                .map_err(Into::<ExecutionError>::into)?;
            Ok(refund)
        })
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
                    .mt_batch_balance_of(account_id, token_ids.into_iter().map(ToString::to_string))
                    .await?;
                anyhow::Ok((account_id, balances))
            })
            .collect::<FuturesOrdered<_>>();

        while let Some((account_id, balances)) = balances.try_next().await.unwrap() {
            println!(
                "{:<64} {}",
                account_id,
                balances.into_iter().map(|b| format!("{:<30}", b)).join(" ")
            );
        }
    }

    pub async fn view_escrow(&self, escrow: &Account) {
        let s = escrow.view_escrow().await.unwrap();
        println!(
            "{}::escrow_view() -> {:#}",
            escrow.id(),
            serde_json::to_value(&s).unwrap()
        );
    }
}
