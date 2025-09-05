use arbitrary::{Arbitrary, Unstructured};
use defuse::core::{
    Deadline, DefuseError,
    intents::{DefuseIntents, tokens::FtWithdraw},
    pack_expirable_nonce,
    token_id::{TokenId, nep141::Nep141TokenId},
};

use defuse_test_utils::{
    asserts::ResultAssertsExt,
    random::{Rng, rng},
};
use near_sdk::json_types::U128;
use rstest::rstest;

use crate::{
    tests::defuse::{
        DefuseSigner, SigningStandard, accounts::AccountManagerExt, env::Env,
        intents::ExecuteIntentsExt,
    },
    utils::mt::MtExt,
};

#[tokio::test]
#[rstest]
async fn test_commit_nonces(#[notrace] mut rng: impl Rng) {
    let env = Env::builder().build().await;
    let current_timestamp = chrono::Utc::now().timestamp_millis() as u64;

    let withdraw_amount: U128 = 1000.into();
    let deposit_amount = withdraw_amount.0 * 3;

    // Create user account

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));
    {
        env.defuse_ft_deposit_to(&env.ft1, deposit_amount, env.user1.id())
            .await
            .unwrap();

        assert_eq!(
            env.mt_contract_balance_of(env.defuse.id(), env.user1.id(), &ft1.to_string())
                .await
                .unwrap(),
            deposit_amount
        );
    }

    // legacy nonce
    let legacy_nonce = rng.random();

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            legacy_nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: env.user1.id().clone(),
                    amount: withdraw_amount,
                    memo: None,
                    msg: None,
                    storage_deposit: None,
                    min_gas: None,
                }
                .into()]
                .into(),
            },
        )])
        .await
        .unwrap();

    assert!(
        env.defuse
            .is_nonce_used(env.user1.id(), &legacy_nonce)
            .await
            .unwrap(),
    );

    // nonce is expired
    let expired_nonce = pack_expirable_nonce(current_timestamp - 10000, &rng.random::<[u8; 22]>());

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            expired_nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: env.user1.id().clone(),
                    amount: withdraw_amount,
                    memo: None,
                    msg: None,
                    storage_deposit: None,
                    min_gas: None,
                }
                .into()]
                .into(),
            },
        )])
        .await
        .assert_err_contains("nonce was already expired");

    // nonce can be committed
    let expirable_nonce =
        pack_expirable_nonce(current_timestamp + 10000, &rng.random::<[u8; 22]>());

    env.defuse
        .execute_intents([env.user1.sign_defuse_message(
            SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
            env.defuse.id(),
            expirable_nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: [FtWithdraw {
                    token: env.ft1.clone(),
                    receiver_id: env.user1.id().clone(),
                    amount: withdraw_amount,
                    memo: None,
                    msg: None,
                    storage_deposit: None,
                    min_gas: None,
                }
                .into()]
                .into(),
            },
        )])
        .await
        .unwrap();

    assert!(
        env.defuse
            .is_nonce_used(env.user1.id(), &expirable_nonce)
            .await
            .unwrap(),
    );
}
