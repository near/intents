mod env;

use std::time::Duration;

use defuse::{
    core::intents::tokens::NotifyOnTransfer,
    tokens::{DepositAction, DepositMessage},
};
use defuse_escrow_swap::{
    ContractStorage, Deadline, OverrideSend, Params, ProtocolFees,
    action::{FillAction, TransferAction, TransferMessage},
    price::Price,
};
use defuse_fees::Pips;
use defuse_sandbox::{
    Account, MtExt, MtViewExt, SigningAccount, TxResult,
    api::types::errors::{DataConversionError, ExecutionError},
};
use defuse_token_id::{TokenId, nep141::Nep141TokenId, nep245::Nep245TokenId};
use futures::{TryStreamExt, stream::FuturesOrdered, try_join};
use impl_tools::autoimpl;
use itertools::Itertools;
use near_sdk::{
    AccountId, AccountIdRef, Gas, NearToken,
    json_types::U128,
    serde_json::{self, json},
    state_init::{StateInit, StateInitV1},
};

use crate::env::{BaseEnv, EscrowExt, EscrowViewExt};

#[tokio::test]
async fn partial_fills() {
    const MAKER_AMOUNT: u128 = 10000;
    const TAKER_AMOUNT: u128 = 20000;

    let env = EscrowEnv::new().await.unwrap();

    // try_join!(
    //     env.src_deposit_to_verifier(env.maker.id(), SRC_TOKEN_ID, MAKER_AMOUNT),
    //     env.dst_deposit_to_verifier(env.takers[0].id(), DST_TOKEN_ID, TAKER_AMOUNT),
    //     env.dst_deposit_to_verifier(env.takers[1].id(), DST_TOKEN_ID, TAKER_AMOUNT),
    //     env.dst_deposit_to_verifier(env.takers[2].id(), DST_TOKEN_ID, TAKER_AMOUNT),
    // )
    // .unwrap();

    let [src_verifier_asset, dst_verifier_asset] =
        [env.src_ft.id().clone(), env.dst_ft.id().clone()]
            .map(Nep141TokenId::new)
            .map(TokenId::from);

    let [src_token, dst_token] = [&src_verifier_asset, &dst_verifier_asset]
        .map(|token_id| {
            Nep245TokenId::new(env.verifier.id().clone(), token_id.to_string()).unwrap()
        })
        .map(Into::<TokenId>::into);

    const TIMEOUT: Duration = Duration::from_secs(60);
    let price: Price = "2".parse().unwrap();

    let params = Params {
        maker: env.maker.id().clone(),

        src_token: src_token.clone(),
        dst_token: dst_token.clone(),

        price,
        deadline: Deadline::timeout(TIMEOUT),

        partial_fills_allowed: true,

        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        // taker_whitelist: Default::default(),
        taker_whitelist: env.takers.iter().map(|a| a.id()).cloned().collect(),
        protocol_fees: ProtocolFees {
            // fee: Pips::ZERO,
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
            .map(|(percent, a)| (a, Pips::from_percent(percent as u32 + 1).unwrap()))
            .skip(1)
            .collect(),

        #[cfg(feature = "auth_call")]
        auth_caller: Some(env.verifier.id().clone()),
        salt: [0; 32],
    };
    let state_init = StateInit::V1(StateInitV1 {
        code: env.escrow_global.clone().into(),
        data: ContractStorage::init_state(&params).unwrap(),
    });

    let escrow = Account::new(
        state_init.derive_account_id(),
        env.root().network_config().clone(),
    );
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
            let refund = env
                .src_ft
                .tx(env.verifier.id().clone())
                .function_call_json::<U128>(
                    "ft_on_transfer",
                    json!({
                        "sender_id": env.maker.id(),
                        "amount": U128(amount),
                        "msg": serde_json::to_string(
                            &DepositMessage::new(escrow.id().clone())
                                .with_action(DepositAction::Notify(
                                    NotifyOnTransfer::new(
                                        serde_json::to_string(&TransferMessage {
                                            params: params.clone(),
                                            action: TransferAction::Fund,
                                        })
                                            .unwrap(),
                                    ).with_state_init(state_init.clone())))
                        ).unwrap()
                    }),
                    Gas::from_tgas(300),
                    NearToken::from_yoctonear(0),
                )
                .await
                .unwrap()
                .0;

            println!("maker sent: {amount}, refund: {refund}");

            env.show_verifier_balances(
                [escrow.id(), env.maker.id()]
                    .into_iter()
                    .chain(env.takers.iter().map(|a| a.id()))
                    .chain(env.fee_collectors.iter().map(|a| a.id()))
                    .map(|a| a.as_ref()),
                &[&src_verifier_asset, &dst_verifier_asset],
            )
            .await;

            assert_eq!(refund, 0);
            env.view_escrow(&escrow).await;
        }
    }

    // takers deposit
    {
        for (taker, amount) in env.takers.iter().zip([10000, 5000, 20000]) {
            let refund = env
                .dst_ft
                .tx(env.verifier.id().clone())
                .function_call_json::<U128>(
                    "ft_on_transfer",
                    json!({
                        "sender_id": taker.id(),
                        "amount": U128(amount),
                        "msg": serde_json::to_string(&DepositMessage::new(escrow.id().clone())
                            .with_action(
                                DepositAction::Notify(NotifyOnTransfer::new(
                                    serde_json::to_string(&TransferMessage {
                                        params: params.clone(),
                                        action: FillAction {
                                            price: "2".parse().unwrap(),
                                            deadline: Deadline::timeout(Duration::from_secs(10)),
                                            receive_src_to: OverrideSend {
                                                memo: Some("taker memo".to_string()),
                                                // msg: Some("taker msg".to_string()),
                                                ..Default::default()
                                            },
                                        }
                                        .into()
                                    })
                                    .unwrap()
                                ))
                            ))
                        .unwrap()
                    }),
                    Gas::from_tgas(300),
                    NearToken::from_yoctonear(0),
                )
                .await
                .unwrap()
                .0;

            println!("taker sent: {amount}, refund: {refund}");

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
            .close_escrow(escrow.id().clone(), params.clone())
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

    src_ft: SigningAccount,
    dst_ft: SigningAccount,

    maker: SigningAccount,
    takers: [SigningAccount; 3],

    fee_collectors: [SigningAccount; 3],
}

impl EscrowEnv {
    pub async fn new() -> TxResult<Self> {
        let env = BaseEnv::new().await?;
        let root = env.root();

        let (
            src_ft,
            dst_ft,
            maker,
            taker1,
            taker2,
            taker3,
            protocol_fee_collector,
            fee_collector2,
            fee_collector3,
        ) = try_join!(
            root.create_subaccount("src-ft", NearToken::from_near(10)),
            root.create_subaccount("dst-ft", NearToken::from_near(10)),
            root.create_subaccount("maker", NearToken::from_near(10)),
            root.create_subaccount("taker1", NearToken::from_near(10)),
            root.create_subaccount("taker2", NearToken::from_near(10)),
            root.create_subaccount("taker3", NearToken::from_near(10)),
            root.create_subaccount("protocol-fee-collector", NearToken::from_near(10)),
            root.create_subaccount("fee-collector2", NearToken::from_near(10)),
            root.create_subaccount("fee-collector3", NearToken::from_near(10)),
        )?;

        Ok(Self {
            env,
            src_ft,
            dst_ft,
            maker,
            takers: [taker1, taker2, taker3],
            fee_collectors: [protocol_fee_collector, fee_collector2, fee_collector3],
        })
    }

    // pub async fn src_deposit_to_verifier(
    //     &self,
    //     receiver_id: &AccountIdRef,
    //     token_id: &str,
    //     amount: u128,
    // ) -> TxResult<u128> {
    //     self.deposit_to_verifier(&self.src_ft, receiver_id, token_id, amount)
    //         .await
    // }

    // pub async fn dst_deposit_to_verifier(
    //     &self,
    //     receiver_id: &AccountIdRef,
    //     token_id: &str,
    //     amount: u128,
    // ) -> TxResult<u128> {
    //     self.deposit_to_verifier(&self.mt_dst, receiver_id, token_id, amount)
    //         .await
    // }

    // async fn deposit_to_verifier(
    //     &self,
    //     mt: &SigningAccount,
    //     receiver_id: &AccountIdRef,
    //     token_id: &str,
    //     amount: u128,
    // ) -> TxResult<u128> {
    //     mt.mt_on_transfer(
    //         mt.id(),
    //         self.verifier.id().clone(),
    //         [(token_id.to_string(), amount)],
    //         receiver_id.to_string(),
    //     )
    //     .await
    //     .and_then(|refunds| {
    //         let [refund] = refunds
    //             .try_into()
    //             .map_err(|refunds: Vec<_>| DataConversionError::IncorrectLength(refunds.len()))
    //             .map_err(Into::<ExecutionError>::into)?;
    //         Ok(refund)
    //     })
    // }

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
        println!("{}: {:?}", escrow.id(), escrow.view().await.unwrap());
        let s = escrow.view_escrow().await.unwrap();
        println!(
            "{}::escrow_view() -> {:#}",
            escrow.id(),
            serde_json::to_value(&s).unwrap()
        );
    }
}
