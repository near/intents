use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use multi_token_receiver_stub::MTReceiverMode as StubAction;

use defuse::{
    core::intents::tokens::NotifyOnTransfer,
    tokens::{DepositAction, DepositMessage, ExecuteIntents},
};
use defuse_sandbox::extensions::ft::FtViewExt;
use defuse_sandbox::extensions::mt::MtViewExt;
use defuse_sandbox::tx::FnCallBuilder;
use defuse_tests::{defuse_signer::DefuseSignerExt, env::Env};
use near_sdk::NearToken;
use near_sdk::serde_json;
use rstest::rstest;

use crate::MT_RECEIVER_STUB_WASM;

use defuse::core::{amounts::Amounts, intents::tokens::Transfer};
use defuse_sandbox::extensions::ft::FtExt;

#[derive(Debug, Clone)]
struct TransferCallExpectation {
    action: StubAction,
    intent_transfer_amount: Option<u128>,
    refund_if_fails: bool,
    expected_sender_ft_balance: u128,
    expected_receiver_mt_balance: u128,
}

#[rstest]
#[case::nothing_to_refund(TransferCallExpectation {
    action: StubAction::ReturnValue(0.into()),
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 0,
    expected_receiver_mt_balance: 1_000,
})]
#[case::partial_refund(TransferCallExpectation {
    action: StubAction::ReturnValue(300.into()),
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 300,
    expected_receiver_mt_balance: 700,
})]
#[case::malicious_refund(TransferCallExpectation {
    action: StubAction::ReturnValue(2_000.into()),
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 1_000,
    expected_receiver_mt_balance: 0,
})]
#[case::receiver_panics_results_with_no_refund(TransferCallExpectation {
    action: StubAction::Panic,
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 1000,
    expected_receiver_mt_balance: 0,
})]
#[case::malicious_receiver(TransferCallExpectation {
    action: StubAction::MaliciousReturn,
    intent_transfer_amount: None,
    refund_if_fails: true,
    expected_sender_ft_balance: 1000,
    expected_receiver_mt_balance: 0,
})]
#[tokio::test]
async fn ft_transfer_call_calls_mt_on_transfer_variants(
    #[case] expectation: TransferCallExpectation,
) {
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user, intent_receiver, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    let receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
        )
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));
    env.initial_ft_storage_deposit(
        vec![user.id(), receiver.id(), intent_receiver.id()],
        vec![ft.id()],
    )
    .await;

    let root = env.sandbox().root();

    root.ft_transfer(ft.id(), user.id(), 1000, None)
        .await
        .unwrap();
    assert_eq!(ft.ft_balance_of(user.id()).await.unwrap(), 1000);

    let intents = match &expectation.intent_transfer_amount {
        Some(amount) => vec![
            receiver
                .sign_defuse_payload_default(
                    &env.defuse,
                    [Transfer {
                        receiver_id: intent_receiver.id().clone(),
                        tokens: Amounts::new(std::iter::once((ft_id.clone(), *amount)).collect()),
                        memo: None,
                        notification: None,
                    }],
                )
                .await
                .unwrap(),
        ],
        None => vec![],
    };

    let deposit_message = if intents.is_empty() {
        DepositMessage {
            receiver_id: receiver.id().clone(),
            action: Some(DepositAction::Notify(NotifyOnTransfer::new(
                serde_json::to_string(&expectation.action).unwrap(),
            ))),
        }
    } else {
        DepositMessage {
            receiver_id: receiver.id().clone(),
            action: Some(DepositAction::Execute(ExecuteIntents {
                execute_intents: intents,
                refund_if_fails: expectation.refund_if_fails,
            })),
        }
    };

    user.ft_transfer_call(
        ft.id(),
        env.defuse.id(),
        1000,
        None,
        &serde_json::to_string(&deposit_message).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        ft.ft_balance_of(user.id()).await.unwrap(),
        expectation.expected_sender_ft_balance
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(receiver.id(), &ft_id.to_string())
            .await
            .unwrap(),
        expectation.expected_receiver_mt_balance
    );
}
