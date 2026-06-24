use defuse_sandbox::{
    assert_eq_defuse_event_logs,
    extensions::{
        DEFAULT_GAS,
        defuse::{
            DefuseExt, DefuseSignerExt, ToEventLog,
            core::{
                PublicKey,
                intents::tokens::NativeWithdraw,
                token_id::{TokenId, nep141::Nep141TokenId},
            },
            tokens::DepositMessage,
        },
        mt::{Mt, MtBalanceOfArgs},
        wnear::WNearExt,
    },
    kit::Final,
};
use near_sdk::NearToken;
use rstest::rstest;

use crate::{
    tests::defuse::env::{Env, env},
    utils::fixtures::{ed25519_pk, secp256k1_pk},
};

#[rstest]
#[tokio::test]
async fn native_withdraw_intent(
    #[future(awt)] env: Env,
    ed25519_pk: PublicKey,
    secp256k1_pk: PublicKey,
) {
    let (user, other_user) = futures::join!(env.create_user(), env.create_user());

    env.initial_ft_storage_deposit(vec![user.account_id(), other_user.account_id()], &[])
        .await;

    let amounts_to_withdraw = [
        // Check for different account_id types
        // See https://github.com/near/nearcore/blob/dcfb6b9fb9f896b839b8728b8033baab963de344/core/parameters/src/cost.rs#L691-L709
        (
            ed25519_pk.to_implicit_account_id(),
            NearToken::from_near(100),
        ),
        (
            secp256k1_pk.to_implicit_account_id(),
            NearToken::from_near(200),
        ),
        (user.account_id().to_owned(), NearToken::from_near(300)),
    ];

    let initial_balances = {
        let mut result = vec![];
        for (account, _) in &amounts_to_withdraw {
            let balance = env
                .account(account)
                .await
                .map_or(NearToken::ZERO, |a| a.amount);

            result.push(balance);
        }
        result
    };

    let total_amount_yocto = amounts_to_withdraw
        .iter()
        .map(|(_, amount)| amount.as_yoctonear())
        .sum();

    env.near_deposit(
        env.wnear.contract_id(),
        NearToken::from_yoctonear(total_amount_yocto),
    )
    .await
    .expect("failed to wrap NEAR");

    env.ft(env.wnear.contract_id())
        .unwrap()
        .transfer_call(
            env.defuse.contract_id(),
            total_amount_yocto,
            DepositMessage::new(other_user.account_id().clone()).to_string(),
        )
        .gas(DEFAULT_GAS)
        .wait_until(Final)
        .await
        .expect("failed to deposit wNEAR to user2");

    // withdraw native NEAR to corresponding receivers
    let withdraw_payload = other_user
        .sign_defuse_payload_default(
            &env.defuse,
            amounts_to_withdraw
                .iter()
                .cloned()
                .map(|(receiver_id, amount)| NativeWithdraw {
                    receiver_id,
                    amount,
                }),
        )
        .await
        .unwrap();

    let (res, _) = env
        .defuse_simulate_and_execute_intents(env.defuse.contract_id(), [withdraw_payload.clone()])
        .await
        .expect("execute_intents: failed to withdraw native NEAR to receivers");

    assert_eq_defuse_event_logs!(withdraw_payload.to_event_log(), res.logs());

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: other_user.account_id(),
                token_id: &TokenId::Nep141(Nep141TokenId::new(env.wnear.contract_id().clone()))
                    .to_string()
            })
            .await
            .unwrap()
            .0,
        0,
        "there should be nothing left deposited for user1"
    );

    // Check balances of NEAR on the blockchain
    for ((receiver_id, amount), initial_balance) in amounts_to_withdraw.iter().zip(initial_balances)
    {
        let balance = env.account(receiver_id).await.unwrap().amount;

        assert!(
            balance == initial_balance.checked_add(*amount).unwrap(),
            "wrong NEAR balance for {receiver_id}: expected minimum {amount}, got {balance}"
        );
    }
}
