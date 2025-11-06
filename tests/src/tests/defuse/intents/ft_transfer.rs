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
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::AccountId;
use rstest::rstest;

use defuse::core::amounts::Amounts;

use crate::tests::defuse::DefuseSignerExt;

#[tokio::test]
#[rstest]
#[trace]
async fn ft_transfer_intent() {
    let env = Env::builder().build().await;

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
async fn ft_transfer_intent_to_defuse() {
    let env = Env::builder().build().await;

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
async fn ft_transfer_intent_to_mt_receiver_smc(
    #[values(
        MTReceiverMode::ReturnValue(500.into()),
        MTReceiverMode::ReturnValue(1500.into()),
        MTReceiverMode::ExceedGasLimit,
        MTReceiverMode::ExceedLogLimit,
        MTReceiverMode::AcceptAll

    )]
    mt_receiver_mode: MTReceiverMode,
) {
    let initial_amount = 1000;

    let env = Env::builder().build().await;

    let (user, ft, mt_receiver) = futures::join!(
        env.create_user(),
        env.create_token(),
        env.deploy_mt_receiver_stub()
    );

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    env.defuse_ft_deposit_to(&ft, initial_amount, user.id())
        .await
        .unwrap();

    let ft1 = TokenId::from(Nep141TokenId::new(ft.clone()));

    let msg = serde_json::to_string(&mt_receiver_mode).unwrap();

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
        msg: Some(msg),
    };

    let transfer_payload = user
        .sign_defuse_payload_default(env.defuse.id(), [transfer_intent])
        .await
        .unwrap();

    env.defuse
        .execute_intents(env.defuse.id(), [transfer_payload])
        .await
        .unwrap();

    match mt_receiver_mode {
        MTReceiverMode::AcceptAll => {
            assert_eq!(
                env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft1.to_string())
                    .await
                    .unwrap(),
                0
            );

            assert_eq!(
                env.mt_contract_balance_of(env.defuse.id(), mt_receiver.id(), &ft1.to_string())
                    .await
                    .unwrap(),
                initial_amount
            );
        }
        MTReceiverMode::ReturnValue(used_amount) if used_amount.0 == 500 => {
            assert_eq!(
                env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft1.to_string())
                    .await
                    .unwrap(),
                initial_amount - used_amount.0
            );

            assert_eq!(
                env.mt_contract_balance_of(env.defuse.id(), mt_receiver.id(), &ft1.to_string())
                    .await
                    .unwrap(),
                used_amount.0
            );
        }

        // in other cases - exceeds gas, exceeds log limit, returns more than transferred - should be refunded
        _ => {
            assert_eq!(
                env.mt_contract_balance_of(env.defuse.id(), user.id(), &ft1.to_string())
                    .await
                    .unwrap(),
                initial_amount
            );

            assert_eq!(
                env.mt_contract_balance_of(env.defuse.id(), mt_receiver.id(), &ft1.to_string())
                    .await
                    .unwrap(),
                0
            );
        }
    }
}
