use std::time::Duration;

use crate::{
    tests::defuse::{
        DefuseSigner, SigningStandard, env::Env, intents::ExecuteIntentsExt,
        tokens::nep141::traits::DefuseFtReceiver,
    },
    utils::{mt::MtExt, wnear::WNearExt},
};
use defuse::{
    core::{
        Deadline,
        crypto::PublicKey,
        intents::{DefuseIntents, tokens::NativeWithdraw},
        token_id::{TokenId, nep141::Nep141TokenId},
    },
    tokens::DepositMessage,
};
use defuse_randomness::Rng;
use defuse_test_utils::random::rng;
use near_sdk::NearToken;
use rstest::rstest;

#[tokio::test]
#[rstest]
async fn native_withdraw_intent(mut rng: impl Rng) {
    let env = Env::new().await;

    let amounts_to_withdraw = [
        // Check for different account_id types
        // See https://github.com/near/nearcore/blob/dcfb6b9fb9f896b839b8728b8033baab963de344/core/parameters/src/cost.rs#L691-L709
        (
            PublicKey::Ed25519(rng.random()).to_implicit_account_id(),
            NearToken::from_near(100),
        ),
        (
            PublicKey::Secp256k1(rng.random()).to_implicit_account_id(),
            NearToken::from_near(200),
        ),
        (env.user1.id().to_owned(), NearToken::from_near(300)),
    ];

    let initial_balances = {
        let mut result = vec![];
        for (account, _) in &amounts_to_withdraw {
            let balance = env
                .sandbox()
                .worker()
                .view_account(account)
                .await
                .map(|a| a.balance)
                .unwrap_or(NearToken::from_near(0));

            result.push(balance);
        }
        result
    };

    let total_amount_yocto = amounts_to_withdraw
        .iter()
        .map(|(_, amount)| amount.as_yoctonear())
        .sum();

    env.near_deposit(
        env.wnear.id(),
        NearToken::from_yoctonear(total_amount_yocto),
    )
    .await
    .expect("failed to wrap NEAR");

    env.defuse_ft_deposit(
        env.defuse.id(),
        env.wnear.id(),
        total_amount_yocto,
        DepositMessage::new(env.user2.id().clone()),
    )
    .await
    .expect("failed to deposit wNEAR to user2");

    // withdraw native NEAR to corresponding receivers
    env.defuse_execute_intents(
        env.defuse.id(),
        [env.user2.sign_defuse_message(
            SigningStandard::default(),
            env.defuse.id(),
            rng.random(),
            Deadline::timeout(Duration::from_secs(120)),
            DefuseIntents {
                intents: amounts_to_withdraw
                    .iter()
                    .cloned()
                    .map(|(receiver_id, amount)| {
                        NativeWithdraw {
                            receiver_id,
                            amount,
                        }
                        .into()
                    })
                    .collect(),
            },
        )],
    )
    .await
    .expect("execute_intents: failed to withdraw native NEAR to receivers");

    assert_eq!(
        env.defuse
            .mt_balance_of(
                env.user1.id(),
                &TokenId::Nep141(Nep141TokenId::new(env.wnear.id().clone())).to_string()
            )
            .await
            .unwrap(),
        0,
        "there should be nothing left deposited for user1"
    );

    // Check balances of NEAR on the blockchain
    for ((receiver_id, amount), initial_balance) in amounts_to_withdraw.iter().zip(initial_balances)
    {
        let balance = env
            .sandbox()
            .worker()
            .view_account(receiver_id)
            .await
            .unwrap()
            .balance;

        assert!(
            balance == initial_balance.checked_add(*amount).unwrap(),
            "wrong NEAR balance for {receiver_id}: expected minimum {amount}, got {balance}"
        );
    }
}
