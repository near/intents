use std::time::Duration;

use defuse::{
    core::{
        Deadline,
        crypto::PublicKey,
        intents::{DefuseIntents, tokens::NativeWithdraw},
        tokens::TokenId,
    },
    tokens::DepositMessage,
};
use near_sdk::NearToken;
use randomness::Rng;
use rstest::rstest;
use test_utils::random::{Seed, make_seedable_rng, random_seed};

use crate::{
    tests::defuse::{
        DefuseSigner, SigningStandard, env::Env, intents::ExecuteIntentsExt,
        tokens::nep141::DefuseFtReceiver,
    },
    utils::{mt::MtExt, wnear::WNearExt},
};

#[tokio::test]
#[rstest]
#[trace]
async fn native_withdraw_intent(random_seed: Seed) {
    let mut rng = make_seedable_rng(random_seed);
    let env = Env::new().await;

    // Check for different account_id types
    // See https://github.com/near/nearcore/blob/dcfb6b9fb9f896b839b8728b8033baab963de344/core/parameters/src/cost.rs#L691-L709
    let receive = [
        (
            PublicKey::Ed25519(rng.random()).to_implicit_account_id(),
            NearToken::from_near(100),
        ),
        (
            PublicKey::Secp256k1(rng.random()).to_implicit_account_id(),
            NearToken::from_near(200),
        ),
        (env.user1.id().clone(), NearToken::from_near(300)),
    ];
    let total_amount_yocto = receive
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
        DepositMessage::new(env.user1.id().clone()),
    )
    .await
    .expect("falied to deposit wNEAR to user1");

    // withdraw native NEAR to corresponding receivers
    env.defuse_execute_intents(
        env.defuse.id(),
        [env.user1.sign_defuse_message(
            SigningStandard::Nep413,
            env.defuse.id(),
            rng.random(),
            Deadline::timeout(Duration::from_secs(120)),
            DefuseIntents {
                intents: receive
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
                &TokenId::Nep141(env.wnear.id().clone()).to_string()
            )
            .await
            .unwrap(),
        0,
        "there should be nothing left deposited for user1"
    );

    for (receiver_id, amount) in receive {
        let balance = env
            .sandbox()
            .worker()
            .view_account(&receiver_id)
            .await
            .unwrap()
            .balance;
        assert!(
            balance >= amount,
            "wrong NEAR balance for {receiver_id}: expected minimum {amount}, got {balance}"
        );
    }
}
