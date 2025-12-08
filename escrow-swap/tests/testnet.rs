use std::time::Duration;

use defuse::{
    core::intents::tokens::NotifyOnTransfer,
    tokens::{DepositAction, DepositMessage},
};
use defuse_deadline::Deadline;
use defuse_escrow_swap::{
    ContractStorage, OverrideSend, Params, ProtocolFees,
    action::{FillAction, TransferAction, TransferMessage},
};
use defuse_fees::Pips;
use defuse_price::Price;
use defuse_token_id::{TokenId, nep141::Nep141TokenId, nep245::Nep245TokenId};
use near_sdk::{
    AccountId, AccountIdRef, GlobalContractId,
    json_types::U128,
    serde_json::{self, json},
    state_init::{StateInit, StateInitV1},
};

#[test]
fn simple() {
    const ROOT: &AccountIdRef = AccountIdRef::new_or_panic("nearseny.testnet");
    let escrow_global_id = subaccount("escrow-swap", ROOT);
    let verifier = subaccount("intents", ROOT);
    let poa_factory = subaccount("omft", ROOT);

    let [src_token_id, dst_token_id] =
        ["src-token", "dst-token"].map(|name| subaccount(name, &poa_factory));

    let [maker, taker] = ["maker", "taker"].map(|name| subaccount(name, ROOT));

    let [src_verifier_token_id, dst_verifier_token_id] =
        [src_token_id.clone(), dst_token_id.clone()]
            .map(Nep141TokenId::new)
            .map(Into::<TokenId>::into);

    let [src_escrow_token_id, dst_escrow_token_id] =
        [&src_verifier_token_id, &dst_verifier_token_id]
            .map(ToString::to_string)
            .map(|token_id| Nep245TokenId::new(verifier.clone(), token_id))
            .map(Into::<TokenId>::into);

    let params = Params {
        maker: maker.clone(),
        src_token: src_escrow_token_id,
        dst_token: dst_escrow_token_id,
        price: "2".parse().unwrap(), // 2 dst per 1 src,
        deadline: Deadline::timeout(Duration::from_secs(60 * 60)), // 1h
        partial_fills_allowed: false,
        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        taker_whitelist: [taker.clone()].into(),
        protocol_fees: Some(ProtocolFees {
            fee: Pips::from_bips(50).unwrap(),
            surplus: Pips::from_percent(20).unwrap(),
            collector: ROOT.to_owned(),
        }),
        integrator_fees: [(
            "integrator.nearseny.near".parse().unwrap(),
            Pips::from_percent(1).unwrap(),
        )]
        .into(),
        auth_caller: Some(verifier.clone()),
        salt: [0; 32],
    };

    let state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(escrow_global_id),
        data: ContractStorage::init_state(&params).unwrap(),
    });

    let escrow_id = state_init.derive_account_id();

    let maker_src_amount = 100_000_000u128;

    println!(
        "{} -> {}::ft_transfer_call({:#})",
        &maker,
        &src_token_id,
        json!({
            "receiver_id": &verifier,
            "amount": U128(maker_src_amount),
            "memo": "fund escrow",
            "msg": serde_json::to_string(
                &DepositMessage::new(escrow_id.clone())
                    .with_action(DepositAction::Notify(
                        NotifyOnTransfer::new(
                            serde_json::to_string(&TransferMessage {
                                params: params.clone(),
                                action: TransferAction::Fund,
                            })
                            .unwrap(),
                        ).with_state_init(state_init)
                )),
            )
            .unwrap(),
        })
    );

    let taker_price: Price = "2.1".parse().unwrap();
    println!(
        "{} -> {}::ft_transfer_call({:#})",
        &taker,
        &dst_token_id,
        json!({
            "receiver_id": &verifier,
            "amount": U128(taker_price.dst_ceil_checked(maker_src_amount).unwrap() + /* excess */ 1_000_000),
            "memo": "fill escrow",
            "msg": serde_json::to_string(
                &DepositMessage::new(escrow_id)
                    .with_action(DepositAction::Notify(
                        NotifyOnTransfer::new(
                            serde_json::to_string(&TransferMessage {
                                params: params.clone(),
                                action: TransferAction::Fill(FillAction {
                                    price: taker_price,
                                    deadline: params.deadline,
                                    receive_src_to: OverrideSend::default(),
                                }),
                            })
                            .unwrap(),
                        )
                )),
            )
            .unwrap(),
        })
    );
}

fn subaccount(name: &str, parent: &AccountIdRef) -> AccountId {
    format!("{name}.{parent}").parse().unwrap()
}
