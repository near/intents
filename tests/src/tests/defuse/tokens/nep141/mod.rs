pub mod traits;

use crate::tests::defuse::SigningStandard;
use crate::tests::defuse::tokens::nep141::traits::DefuseFtWithdrawer;
use crate::{
    tests::{
        defuse::{DefuseSigner, env::Env},
        poa::factory::PoAFactoryExt,
    },
    utils::{acl::AclExt, ft::FtExt, mt::MtExt},
};
use arbitrary::{Arbitrary, Unstructured};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;

use defuse::{
    contract::Role,
    core::{
        Deadline,
        intents::{DefuseIntents, tokens::FtWithdraw},
    },
    tokens::DepositMessage,
};
use defuse_randomness::Rng;
use defuse_test_utils::random::rng;
use near_sdk::json_types::U128;
use rstest::rstest;
use std::time::Duration;

#[tokio::test]
#[rstest]
#[trace]
async fn deposit_withdraw(#[values(false, true)] no_registration: bool) {
    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let user = env.create_user("user").await;
    let ft = env.create_token("ft").await;

    env.deposit_to_users(vec![user.id()], &[&ft]).await;

    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        user.defuse_ft_withdraw(env.defuse.id(), &ft, user.id(), 1000, None, None)
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 1000);
}

#[tokio::test]
#[rstest]
async fn poa_deposit(#[values(false, true)] no_registration: bool) {
    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let user = env.create_user("user").await;

    let ft = env.create_token("ft").await;
    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    env.deposit_to_users(vec![user.id()], &[&ft]).await;

    env.poa_factory_ft_deposit(
        env.poa_factory.id(),
        &env.poa_ft_name(&ft),
        user.id(),
        1000,
        Some(DepositMessage::new(user.id().clone()).to_string()),
        None,
    )
    .await
    .unwrap();

    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 0);
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn deposit_withdraw_intent(
    #[notrace] mut rng: impl Rng,
    #[values(false, true)] no_registration: bool,
) {
    use crate::tests::defuse::tokens::nep141::traits::DefuseFtReceiver;

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let user = env.create_user("user").await;
    let other_user = env.create_user("other_user").await;

    let ft = env.create_token("ft").await;

    env.poa_factory_ft_deposit(
        env.poa_factory.id(),
        &env.poa_ft_name(&ft),
        user.id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let nonce = rng.random();

    assert_eq!(
        user.defuse_ft_deposit(
            env.defuse.id(),
            &ft,
            1000,
            DepositMessage {
                receiver_id: user.id().clone(),
                execute_intents: [user.sign_defuse_message(
                    SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>()))
                        .unwrap(),
                    env.defuse.id(),
                    nonce,
                    Deadline::timeout(Duration::from_secs(120)),
                    DefuseIntents {
                        intents: [
                            // withdrawal is a detached promise
                            FtWithdraw {
                                token: ft.clone(),
                                receiver_id: other_user.id().clone(),
                                amount: U128(600),
                                memo: None,
                                msg: None,
                                storage_deposit: None,
                                min_gas: None,
                            }
                            .into(),
                        ]
                        .into(),
                    },
                )]
                .into(),
                // another promise will be created for `execute_intents()`
                refund_if_fails: false,
            },
        )
        .await
        .unwrap(),
        1000
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 0);
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        400
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), other_user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        env.ft_token_balance_of(&ft, other_user.id()).await.unwrap(),
        600
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn deposit_withdraw_intent_refund(
    #[notrace] mut rng: impl Rng,
    #[values(false, true)] no_registration: bool,
) {
    use arbitrary::{Arbitrary, Unstructured};

    use crate::tests::defuse::{SigningStandard, tokens::nep141::traits::DefuseFtReceiver};

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let user = env.create_user("user").await;
    let ft = env.create_token("ft").await;

    env.deposit_to_users(vec![user.id()], &[&ft]).await;

    env.poa_factory_ft_deposit(
        env.poa_factory.id(),
        &env.poa_ft_name(&ft),
        user.id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let nonce = rng.random();

    assert_eq!(
        user.defuse_ft_deposit(
            env.defuse.id(),
            &ft,
            1000,
            DepositMessage {
                receiver_id: user.id().clone(),
                execute_intents: [user.sign_defuse_message(
                    SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>()))
                        .unwrap(),
                    env.defuse.id(),
                    nonce,
                    Deadline::MAX,
                    DefuseIntents {
                        intents: [FtWithdraw {
                            token: ft.clone(),
                            receiver_id: user.id().clone(),
                            amount: U128(1001),
                            memo: None,
                            msg: None,
                            storage_deposit: None,
                            min_gas: None,
                        }
                        .into(),]
                        .into(),
                    },
                )]
                .into(),
                refund_if_fails: true,
            },
        )
        .await
        .unwrap(),
        0
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));
    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 1000);
}

#[tokio::test]
#[rstest]
async fn ft_force_withdraw(#[values(false, true)] no_registration: bool) {
    use defuse::core::token_id::nep141::Nep141TokenId;

    use crate::tests::defuse::tokens::nep141::traits::DefuseFtWithdrawer;

    let env = Env::builder()
        .deployer_as_super_admin()
        .no_registration(no_registration)
        .build()
        .await;

    let user = env.create_user("user").await;
    let other_user = env.create_user("other_user").await;

    let ft = env.create_token("ft").await;

    env.deposit_to_users(vec![user.id()], &[&ft]).await;

    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    other_user
        .defuse_ft_force_withdraw(
            env.defuse.id(),
            user.id(),
            &ft,
            other_user.id(),
            1000,
            None,
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        env.ft_token_balance_of(&ft, other_user.id()).await.unwrap(),
        0
    );

    env.acl_grant_role(
        env.defuse.id(),
        Role::UnrestrictedWithdrawer,
        other_user.id(),
    )
    .await
    .unwrap();

    assert_eq!(
        other_user
            .defuse_ft_force_withdraw(
                env.defuse.id(),
                user.id(),
                &ft,
                other_user.id(),
                1000,
                None,
                None,
            )
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        env.ft_token_balance_of(&ft, other_user.id()).await.unwrap(),
        1000
    );
}
