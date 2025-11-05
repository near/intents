pub mod traits;

use crate::tests::defuse::tokens::nep141::traits::{DefuseFtReceiver, DefuseFtWithdrawer};
use crate::{
    tests::{
        defuse::env::{Env, MT_RECEIVER_STUB_WASM},
        poa::factory::PoAFactoryExt,
    },
    utils::{acl::AclExt, ft::FtExt, mt::MtExt},
};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;

use defuse::{contract::Role, core::intents::tokens::FtWithdraw, tokens::DepositMessage};
use near_sdk::json_types::U128;
use rstest::rstest;

#[tokio::test]
#[rstest]
#[trace]
async fn deposit_withdraw(#[values(false, true)] no_registration: bool) {
    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

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

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

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
async fn deposit_withdraw_intent(#[values(false, true)] no_registration: bool) {
    use crate::tests::defuse::{DefuseSignerExt, tokens::nep141::traits::DefuseFtReceiver};

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id(), other_user.id()], vec![&ft])
        .await;

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

    let withdraw_intent_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [FtWithdraw {
                token: ft.clone(),
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
            &ft,
            1000,
            DepositMessage {
                receiver_id: user.id().clone(),
                execute_intents: [withdraw_intent_payload].into(),
                // another promise will be created for `execute_intents()`
                refund_if_fails: false,
                message: String::new(),
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
async fn deposit_withdraw_intent_refund(#[values(false, true)] no_registration: bool) {
    use crate::tests::defuse::{DefuseSignerExt, tokens::nep141::traits::DefuseFtReceiver};

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

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

    let overflow_withdraw_payload = user
        .sign_defuse_payload_default(
            env.defuse.id(),
            [FtWithdraw {
                token: ft.clone(),
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
            &ft,
            1000,
            DepositMessage {
                receiver_id: user.id().clone(),
                execute_intents: [overflow_withdraw_payload].into(),
                refund_if_fails: true,
                message: String::new(),
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

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.id(), other_user.id()], vec![&ft])
        .await;

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

#[tokio::test]
async fn ft_transfer_call_stub_action_message() {
    use crate::utils::account::AccountExt;
    use multi_token_receiver_stub::StubAction;
    use near_sdk::serde_json;

    let env = Env::builder()
        .deployer_as_super_admin()
        .no_registration(false)
        .build()
        .await;

    let (user, receiver, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());


    println!("deploying stub contract");
    receiver.deploy(MT_RECEIVER_STUB_WASM.as_slice()).await.unwrap().unwrap();
    println!("deployed!!!");


    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    println!("ft_id: {}", ft.to_string());
    println!("user_id: {}", user.id().to_string());
    println!("receiver_id: {}", receiver.id().to_string());
    env.initial_ft_storage_deposit(vec![user.id(), receiver.id()], vec![&ft])
        .await;

    // env.defuse_ft_deposit_to(&ft, 1000, user.id())
    //     .await
    //     .unwrap();

    let root = env.sandbox().root_account();
    assert!(env.ft_token_balance_of(&ft, root.id()).await.unwrap() > 0);

    // root.ft_transfer_call(&ft, user.id(), 1000, None, "").await.unwrap();

    root.ft_transfer(&ft, user.id(), 1000, None).await.unwrap();
    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 1000);

    // user.ft_transfer(&ft, receiver.id(), 1000, None)
    //     .await
    //     .unwrap();
    //



    let deposit_message = DepositMessage::new(receiver.id().clone())
        .with_refund_if_fails()
        .with_message(serde_json::to_string(&StubAction::ReturnValue(100.into())).unwrap());

    user.ft_transfer_call(
        &ft,
        env.defuse.id(),
        800,
        None,
        &serde_json::to_string(&deposit_message).unwrap(),
    ).await.unwrap();

    assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 300);

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), receiver.id(), &ft_id.to_string())
            .await
            .unwrap(),
        700
    );

    // assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 0);
    // assert_eq!(env.ft_token_balance_of(&ft, receiver.id()).await.unwrap(), 1000);

    // user.ft_transfer(&ft, user.id(), 1000, None).await.unwrap();

    // env.ft_transfer_call(&ft, user.id(), 1000, None, "")
    //     .await
    //     .unwrap();


    //
    // let deposit_message = DepositMessage {
    //     receiver_id: receiver.id().clone(),
    //     execute_intents: Vec::new(),
    //     refund_if_fails: false,
    //     // message: serde_json::to_string(&StubAction::ReturnValue(U128(0u128))).unwrap(),
    //     message: String::new(),
    // };
    //
    // let result = user
    //     .defuse_ft_deposit(env.defuse.id(), &ft, 1000, Some(deposit_message))
    //     .await
    //     .expect("ft_transfer_call should succeed");
    //
    //
    // assert_eq!(
    //     env.mt_contract_balance_of(env.defuse.id(), receiver.id(), &ft_id.to_string())
    //         .await
    //         .unwrap(),
    //     1000
    // );
    //
    // assert_eq!(env.ft_token_balance_of(&ft, user.id()).await.unwrap(), 0);
}
