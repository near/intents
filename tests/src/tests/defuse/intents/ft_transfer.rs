use super::ExecuteIntentsExt;
use crate::tests::defuse::DefuseExt;
use crate::{
    tests::defuse::env::Env,
    utils::{ft::FtExt, mt::MtExt},
};
use defuse::core::intents::tokens::Transfer;
use defuse::core::token_id::nep245::Nep245TokenId;
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::{FeesConfig, Pips},
};
use near_sdk::AccountId;
use rstest::rstest;

use defuse::core::amounts::Amounts;

use crate::tests::defuse::DefuseSignerExt;

#[tokio::test]
#[rstest]
#[trace]
async fn ft_transfer_intent(#[values(false, true)] no_registration: bool) {
    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let other_user_id: AccountId = "other-user.near".parse().unwrap();
    let token_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let transfer_intent = Transfer {
        receiver_id: other_user_id.clone(),
        tokens: Amounts::new(
            std::iter::once((TokenId::from(Nep141TokenId::new(ft.clone())), 1000)).collect(),
        ),
        memo: None,
        msg: None,
    };

    let initial_transfer_payload = user
        .sign_defuse_payload_default(env.defuse.id(), [transfer_intent])
        .await
        .unwrap();

    env.defuse
        .execute_intents(env.defuse.id(), [initial_transfer_payload])
        .await
        .unwrap();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &token_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), &other_user_id, &token_id.to_string())
            .await
            .unwrap(),
        1000
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn ft_transfer_intent_to_defuse(#[values(false, true)] no_registration: bool) {
    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());
    let other_user_id: AccountId = "other-user.near".parse().unwrap();

    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.id().clone(),
                },
                roles: RolesConfig::default(),
            },
            false,
        )
        .await
        .unwrap();

    env.initial_ft_storage_deposit(vec![user.id(), defuse2.id()], vec![&ft])
        .await;

    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(ft.clone()));

    let transfer_intent = Transfer {
        receiver_id: defuse2.id().clone(),
        tokens: Amounts::new(
            std::iter::once((TokenId::from(Nep141TokenId::new(ft.clone())), 1000)).collect(),
        ),
        memo: None,
        msg: Some(other_user_id.to_string()),
    };

    let transfer_payload = user
        .sign_defuse_payload_default(env.defuse.id(), [transfer_intent])
        .await
        .unwrap();

    assert!(user.mt_tokens(defuse2.id(), ..).await.unwrap().is_empty());

    env.defuse
        .execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft1.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), defuse2.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000
    );

    assert_eq!(user.mt_tokens(defuse2.id(), ..).await.unwrap().len(), 1);
    assert_eq!(
        user.mt_tokens_for_owner(defuse2.id(), &other_user_id, ..)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(env.ft_token_balance_of(&ft, defuse2.id()).await.unwrap(), 0);

    let defuse_ft1 =
        TokenId::from(Nep245TokenId::new(env.defuse.id().clone(), ft1.to_string()).unwrap());

    assert_eq!(
        env.mt_contract_balance_of(defuse2.id(), &other_user_id, &defuse_ft1.to_string())
            .await
            .unwrap(),
        1000
    );

    assert_eq!(
        env.ft_token_balance_of(&ft, env.defuse.id()).await.unwrap(),
        1000
    );

    assert_eq!(env.ft_token_balance_of(&ft, defuse2.id()).await.unwrap(), 0);
}

#[tokio::test]
#[rstest]
#[trace]
async fn ft_transfer_intent_to_mt_receiver_smc(#[values(false, true)] no_registration: bool) {
    use crate::utils::account::AccountExt;

    let initial_amount = 1000;
    let used_amount = 500;

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    const MT_RECEIVER_CODE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/contracts/shy_receiver.wasm"
    ));

    let mt_receiver = env
        .sandbox()
        .root_account()
        .deploy_contract("shy_receiver", MT_RECEIVER_CODE)
        .await
        .unwrap();

    // let mt_receiver = env
    //     .deploy_defuse(
    //         "defuse2",
    //         DefuseConfig {
    //             wnear_id: env.wnear.id().clone(),
    //             fees: FeesConfig {
    //                 fee: Pips::ZERO,
    //                 fee_collector: env.id().clone(),
    //             },
    //             roles: RolesConfig::default(),
    //         },
    //         false,
    //     )
    //     .await
    //     .unwrap();

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    env.defuse_ft_deposit_to(&ft, initial_amount, user.id())
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(ft.clone()));

    let transfer_intent = Transfer {
        receiver_id: mt_receiver.id().clone(),
        tokens: Amounts::new(
            std::iter::once((
                TokenId::from(Nep141TokenId::new(ft.clone())),
                initial_amount,
            ))
            .collect(),
        ),
        memo: None,
        msg: Some(used_amount.to_string()),
    };

    let transfer_payload = user
        .sign_defuse_payload_default(env.defuse.id(), [transfer_intent])
        .await
        .unwrap();

    env.defuse
        .execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft1.to_string())
            .await
            .unwrap(),
        initial_amount - used_amount
    );

    assert_eq!(
        env.mt_contract_balance_of(env.defuse.id(), mt_receiver.id(), &ft1.to_string())
            .await
            .unwrap(),
        used_amount
    );
}
