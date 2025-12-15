use crate::{DefuseSignerExt, env::Env};
use defuse::{core::intents::tokens::StorageDeposit, sandbox_ext::intents::ExecuteIntentsExt};
use defuse_sandbox::extensions::{
    storage_management::{StorageManagementExt, StorageViewExt},
    wnear::WNearExt,
};
use near_sdk::NearToken;
use rstest::rstest;

const MIN_FT_STORAGE_DEPOSIT_VALUE: NearToken =
    NearToken::from_yoctonear(1_250_000_000_000_000_000_000);

const ONE_YOCTO_NEAR: NearToken = NearToken::from_yoctonear(1);

#[rstest]
#[trace]
#[case(MIN_FT_STORAGE_DEPOSIT_VALUE, Some(MIN_FT_STORAGE_DEPOSIT_VALUE))]
#[trace]
#[case(
    MIN_FT_STORAGE_DEPOSIT_VALUE.checked_sub(ONE_YOCTO_NEAR).unwrap(), // Sending less than the required min leads to nothing being deposited
    None
)]
#[trace]
#[case(
    MIN_FT_STORAGE_DEPOSIT_VALUE.checked_add(ONE_YOCTO_NEAR).unwrap(),
    Some(MIN_FT_STORAGE_DEPOSIT_VALUE)
)]
#[tokio::test]
async fn storage_deposit_success(
    #[case] amount_to_deposit: NearToken,
    #[case] expected_deposited: Option<NearToken>,
) {
    let env = Env::builder().disable_ft_storage_deposit().build().await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.fund_account_with_near(user.id(), NearToken::from_near(1000))
        .await
        .unwrap();
    env.fund_account_with_near(other_user.id(), NearToken::from_near(1000))
        .await
        .unwrap();
    {
        let storage_balance_ft1_user1 = ft.storage_balance_of(user.id()).await.unwrap();

        let storage_balance_ft1_user2 = ft.storage_balance_of(other_user.id()).await.unwrap();

        assert!(storage_balance_ft1_user1.is_none());
        assert!(storage_balance_ft1_user2.is_none());
    }

    // For intents contract to have a balance in wnear, we make a storage deposit for it
    env.storage_deposit(
        env.wnear.id(),
        env.defuse.id().as_ref(),
        NearToken::from_near(1),
    )
    .await
    .unwrap();

    env.storage_deposit(ft.id(), user.id().as_ref(), NearToken::from_near(1))
        .await
        .unwrap();

    {
        let storage_balance_ft1_user1 = ft.storage_balance_of(user.id()).await.unwrap();

        let storage_balance_ft1_user2 = ft.storage_balance_of(other_user.id()).await.unwrap();

        assert_eq!(
            storage_balance_ft1_user1.unwrap().total,
            MIN_FT_STORAGE_DEPOSIT_VALUE
        );
        assert!(storage_balance_ft1_user2.is_none());
    }

    // The user should have enough wnear in his account (in his account in the wnear contract)
    other_user
        .near_deposit(env.wnear.id(), NearToken::from_near(100))
        .await
        .unwrap();

    // Fund the user's account with near in the intents contract for the storage deposit intent
    env.defuse_ft_deposit_to(
        env.wnear.id(),
        NearToken::from_near(10).as_yoctonear(),
        other_user.id(),
        None,
    )
    .await
    .unwrap();

    let storage_deposit_payload = other_user
        .sign_defuse_payload_default(
            &env.defuse,
            [StorageDeposit {
                contract_id: ft.id().clone(),
                deposit_for_account_id: other_user.id().clone(),
                amount: amount_to_deposit,
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [storage_deposit_payload])
        .await
        .unwrap();

    {
        let storage_balance_ft1_user2 = ft.storage_balance_of(other_user.id()).await.unwrap();

        assert_eq!(
            storage_balance_ft1_user2.map(|v| v.total),
            expected_deposited
        );
    }
}

#[rstest]
#[tokio::test]
async fn storage_deposit_fails_user_has_no_balance_in_intents() {
    let env = Env::builder().disable_ft_storage_deposit().build().await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.fund_account_with_near(user.id(), NearToken::from_near(1000))
        .await
        .unwrap();
    env.fund_account_with_near(other_user.id(), NearToken::from_near(1000))
        .await
        .unwrap();
    {
        let storage_balance_ft1_user1 = ft.storage_balance_of(user.id()).await.unwrap();

        let storage_balance_ft1_user2 = ft.storage_balance_of(other_user.id()).await.unwrap();

        assert!(storage_balance_ft1_user1.is_none());
        assert!(storage_balance_ft1_user2.is_none());
    }

    // For intents contract to have a balance in wnear, we make a storage deposit for it
    env.storage_deposit(
        env.wnear.id(),
        Some(env.defuse.id().as_ref()),
        NearToken::from_near(1),
    )
    .await
    .unwrap();

    env.storage_deposit(ft.id(), user.id().as_ref(), NearToken::from_near(1))
        .await
        .unwrap();

    {
        let storage_balance_ft1_user1 = ft.storage_balance_of(user.id()).await.unwrap();

        let storage_balance_ft1_user2 = ft.storage_balance_of(other_user.id()).await.unwrap();

        assert_eq!(
            storage_balance_ft1_user1.unwrap().total,
            MIN_FT_STORAGE_DEPOSIT_VALUE
        );
        assert!(storage_balance_ft1_user2.is_none());
    }

    // The user should have enough wnear in his account (in his account in the wnear contract)
    other_user
        .near_deposit(env.wnear.id(), NearToken::from_near(100))
        .await
        .unwrap();

    let signed_intents = [other_user
        .sign_defuse_payload_default(
            &env.defuse,
            [StorageDeposit {
                contract_id: ft.id().clone(),
                deposit_for_account_id: other_user.id().clone(),
                amount: MIN_FT_STORAGE_DEPOSIT_VALUE,
            }],
        )
        .await
        .unwrap()];

    // Fails because the user does not own any wNEAR in the intents smart contract. They should first deposit wNEAR.
    env.simulate_and_execute_intents(env.defuse.id(), signed_intents)
        .await
        .unwrap_err();
}
