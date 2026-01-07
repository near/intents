use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse_tests::{
    contract_extensions::{
        defuse::tokens::nep141::{DefuseFtDepositor, DefuseFtWithdrawer},
        poa::PoAFactoryExt,
    },
    defuse_signer::DefuseSignerExt,
    env::Env,
    sandbox::extensions::{acl::AclExt, ft::FtViewExt, mt::MtViewExt},
};

use defuse::{
    contract::Role,
    core::intents::tokens::FtWithdraw,
    tokens::{DepositAction, DepositMessage, ExecuteIntents},
};

use near_sdk::json_types::U128;
use rstest::rstest;

#[rstest]
#[trace]
#[tokio::test]
async fn deposit_withdraw() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        user.defuse_ft_withdraw(env.defuse.id(), ft.id(), user.id(), 1000, None, None)
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(ft.ft_balance_of(user.id()).await.unwrap(), 1000);
}

#[rstest]
#[tokio::test]
async fn poa_deposit() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    env.poa_factory_ft_deposit(
        env.poa_factory.id(),
        &env.poa_ft_name(ft.id()),
        user.id(),
        1000,
        Some(DepositMessage::new(user.id().clone()).to_string()),
        None,
    )
    .await
    .unwrap();

    assert_eq!(ft.ft_balance_of(user.id()).await.unwrap(), 0);
    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn deposit_withdraw_intent() {
    let env = Env::builder().build().await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id(), other_user.id()], vec![ft.id()])
        .await;

    env.poa_factory_ft_deposit(
        env.poa_factory.id(),
        &env.poa_ft_name(ft.id()),
        user.id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let withdraw_intent_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.id().clone(),
                receiver_id: other_user.id().clone(),
                amount: U128(600),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    assert_eq!(
        user.defuse_ft_deposit(
            env.defuse.id(),
            ft.id(),
            1000,
            DepositMessage {
                receiver_id: user.id().clone(),
                action: Some(DepositAction::Execute(ExecuteIntents {
                    execute_intents: [withdraw_intent_payload].into(),
                    // another promise will be created for `execute_intents()`
                    refund_if_fails: false,
                })),
            },
        )
        .await
        .unwrap(),
        1000
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    assert_eq!(ft.ft_balance_of(user.id()).await.unwrap(), 0);
    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        400
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(other_user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(ft.ft_balance_of(other_user.id()).await.unwrap(), 600);
}

#[rstest]
#[trace]
#[tokio::test]
async fn deposit_withdraw_intent_refund() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    env.poa_factory_ft_deposit(
        env.poa_factory.id(),
        &env.poa_ft_name(ft.id()),
        user.id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let overflow_withdraw_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [FtWithdraw {
                token: ft.id().clone(),
                receiver_id: user.id().clone(),
                amount: U128(1001),
                memo: None,
                msg: None,
                storage_deposit: None,
                min_gas: None,
            }],
        )
        .await
        .unwrap();

    assert_eq!(
        user.defuse_ft_deposit(
            env.defuse.id(),
            ft.id(),
            1000,
            DepositMessage {
                receiver_id: user.id().clone(),
                action: Some(DepositAction::Execute(ExecuteIntents {
                    execute_intents: [overflow_withdraw_payload].into(),
                    refund_if_fails: true,
                })),
            },
        )
        .await
        .unwrap(),
        0
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));
    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(ft.ft_balance_of(user.id()).await.unwrap(), 1000);
}

#[rstest]
#[tokio::test]
async fn ft_force_withdraw() {
    use defuse::core::token_id::nep141::Nep141TokenId;

    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id(), other_user.id()], vec![ft.id()])
        .await;

    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    other_user
        .defuse_ft_force_withdraw(
            env.defuse.id(),
            user.id(),
            ft.id(),
            other_user.id(),
            1000,
            None,
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000
    );
    assert_eq!(ft.ft_balance_of(other_user.id()).await.unwrap(), 0);

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
                ft.id(),
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
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
    assert_eq!(ft.ft_balance_of(other_user.id()).await.unwrap(), 1000);
}
