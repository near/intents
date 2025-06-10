use std::time::Duration;

use defuse::core::{
    Deadline,
    intents::{DefuseIntents, tokens::NativeWithdraw},
    tokens::TokenId,
};
use near_sdk::{AccountId, NearToken};
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
    const AMOUNT: NearToken = NearToken::from_near(10);

    let mut rng = make_seedable_rng(random_seed);
    let env = Env::new().await;

    env.transfer_near(env.user1.id(), AMOUNT)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    env.user1
        .near_deposit(env.wnear.id(), AMOUNT)
        .await
        .unwrap();
    env.user1
        .defuse_ft_deposit(env.defuse.id(), env.wnear.id(), AMOUNT.as_yoctonear(), None)
        .await
        .unwrap();

    let receiver_id: AccountId = hex::encode(env.user1.secret_key().public_key().key_data())
        .parse()
        .unwrap();

    env.defuse_execute_intents(
        env.defuse.id(),
        [env.user1.sign_defuse_message(
            SigningStandard::Nep413,
            env.defuse.id(),
            rng.random(),
            Deadline::timeout(Duration::from_secs(120)),
            DefuseIntents {
                intents: [NativeWithdraw {
                    receiver_id: receiver_id.clone(),
                    amount: AMOUNT,
                }
                .into()]
                .into(),
            },
        )],
    )
    .await
    .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(
                env.user1.id(),
                &TokenId::Nep141(env.wnear.id().clone()).to_string()
            )
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        env.sandbox()
            .worker()
            .view_account(&receiver_id)
            .await
            .unwrap()
            .balance,
        AMOUNT,
    );
}
