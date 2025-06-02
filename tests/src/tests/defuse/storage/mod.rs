use crate::{
    tests::defuse::{DefuseSigner, SigningStandard, env::Env, intents::ExecuteIntentsExt},
    utils::{storage_management::StorageManagementExt, wnear::WNearExt},
};
use arbitrary::{Arbitrary, Unstructured};
use defuse::core::Deadline;
use defuse::core::intents::{DefuseIntents, tokens::StorageDeposit};
use near_sdk::NearToken;
use randomness::Rng;
use rstest::rstest;
use test_utils::random::random_seed;
use test_utils::random::{Seed, make_seedable_rng};

const MIN_FT_STORAGE_DEPOSIT_VALUE: NearToken =
    NearToken::from_yoctonear(1_250_000_000_000_000_000_000);

const ONE_YOCTO_NEAR: NearToken = NearToken::from_yoctonear(1);

#[tokio::test]
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
async fn storage_deposit_success(
    random_seed: Seed,
    #[case] amount_to_deposit: NearToken,
    #[case] expected_deposited: Option<NearToken>,
) {
    let mut rng = make_seedable_rng(random_seed);

    let env = Env::builder()
        .disable_ft_storage_deposit()
        .no_registration(false)
        .build()
        .await;

    env.fund_account_with_near(env.user1.id(), NearToken::from_near(1000))
        .await;
    env.fund_account_with_near(env.user2.id(), NearToken::from_near(1000))
        .await;
    env.fund_account_with_near(env.defuse.id(), NearToken::from_near(10000))
        .await;

    {
        let storage_balance_ft1_user1 = env
            .storage_balance_of(&env.ft1, env.user1.id())
            .await
            .unwrap();

        let storage_balance_ft1_user2 = env
            .storage_balance_of(&env.ft1, env.user2.id())
            .await
            .unwrap();

        assert!(storage_balance_ft1_user1.is_none());
        assert!(storage_balance_ft1_user2.is_none());
    }

    // For intents contract to have a balance in wnear, we make a storage deposit for it
    env.poa_factory
        .storage_deposit(
            env.wnear.id(),
            Some(env.defuse.id()),
            NearToken::from_near(1),
        )
        .await
        .unwrap();

    env.poa_factory
        .storage_deposit(&env.ft1, Some(env.user1.id()), NearToken::from_near(1))
        .await
        .unwrap();

    {
        let storage_balance_ft1_user1 = env
            .storage_balance_of(&env.ft1, env.user1.id())
            .await
            .unwrap();

        let storage_balance_ft1_user2 = env
            .storage_balance_of(&env.ft1, env.user2.id())
            .await
            .unwrap();

        assert_eq!(
            storage_balance_ft1_user1.unwrap().total,
            MIN_FT_STORAGE_DEPOSIT_VALUE
        );
        assert!(storage_balance_ft1_user2.is_none());
    }

    // The user should have enough wnear in his account (in his account in the wnear contract)
    env.user2
        .near_deposit(env.wnear.id(), NearToken::from_near(100))
        .await
        .unwrap();

    // Fund the user's account with near in the intents contract for the storage deposit intent
    env.defuse_ft_deposit_to(
        env.wnear.id(),
        NearToken::from_near(10).as_yoctonear(),
        env.user2.id(),
    )
    .await
    .unwrap();

    let nonce = rng.random();

    let intents = [env.user2.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce,
        Deadline::timeout(std::time::Duration::from_secs(120)),
        DefuseIntents {
            intents: [StorageDeposit {
                contract_id: env.ft1.clone(),
                account_id: env.user2.id().clone(),
                amount: amount_to_deposit,
            }
            .into()]
            .into(),
        },
    )];

    // Test events emitted from the simulation
    {
        let simulation_output = env.defuse.simulate_intents(intents.clone()).await.unwrap();

        // Expecting two events, one for withdrawal, and one for execution of the intent
        assert_eq!(simulation_output.emitted_events.len(), 2);
        {
            let withdraw_event = simulation_output
                .emitted_events
                .iter()
                .find(|v| v.get("event").unwrap() == "storage_deposit")
                .unwrap();

            assert_eq!(withdraw_event.get("standard").unwrap(), "dip4");
            assert!(withdraw_event.get("version").is_some());

            let data = withdraw_event.get("data").unwrap().as_array().unwrap();
            assert_eq!(data.len(), 1);
            let data = data.first().unwrap();

            assert_eq!(data.get("account_id").unwrap(), &env.user2.id().to_string());
            assert_eq!(data.get("contract_id").unwrap(), &env.ft1.to_string());
            // This result does not make sense with respect to other contracts, because
            // other FT contracts only accept 0.00125 NEAR for storage deposits.
            // But this is just for this contract's simulation.
            assert_eq!(
                data.get("amount").unwrap(),
                &amount_to_deposit.as_yoctonear().to_string()
            );
        }

        {
            let intent_exec_event = simulation_output
                .emitted_events
                .iter()
                .find(|v| v.get("event").unwrap() == "intents_executed")
                .unwrap();
            assert_eq!(intent_exec_event.get("standard").unwrap(), "dip4");
            assert!(intent_exec_event.get("version").is_some());

            let data = intent_exec_event.get("data").unwrap().as_array().unwrap();
            assert_eq!(data.len(), 1);
            let data = data.first().unwrap();

            assert_eq!(data.get("account_id").unwrap(), &env.user2.id().to_string());
        }

        simulation_output.into_result().unwrap();
    }

    env.defuse.execute_intents(intents).await.unwrap();

    {
        let storage_balance_ft1_user2 = env
            .storage_balance_of(&env.ft1, env.user2.id())
            .await
            .unwrap();

        assert_eq!(
            storage_balance_ft1_user2.map(|v| v.total),
            expected_deposited
        );
    }
}

#[tokio::test]
#[rstest]
#[trace]
async fn storage_deposit_fails_user_has_no_balance_in_intents(random_seed: Seed) {
    let mut rng = make_seedable_rng(random_seed);

    let env = Env::builder()
        .disable_ft_storage_deposit()
        .no_registration(false)
        .build()
        .await;

    env.fund_account_with_near(&env.user1.id().to_owned(), NearToken::from_near(1000))
        .await;
    env.fund_account_with_near(&env.user2.id().to_owned(), NearToken::from_near(1000))
        .await;
    env.fund_account_with_near(&env.defuse.id().to_owned(), NearToken::from_near(10000))
        .await;

    {
        let storage_balance_ft1_user1 = env
            .storage_balance_of(&env.ft1, env.user1.id())
            .await
            .unwrap();

        let storage_balance_ft1_user2 = env
            .storage_balance_of(&env.ft1, env.user2.id())
            .await
            .unwrap();

        assert!(storage_balance_ft1_user1.is_none());
        assert!(storage_balance_ft1_user2.is_none());
    }

    // For intents contract to have a balance in wnear, we make a storage deposit for it
    env.storage_deposit(
        env.wnear.id(),
        Some(env.defuse.id()),
        NearToken::from_near(1),
    )
    .await
    .unwrap();

    env.poa_factory
        .storage_deposit(&env.ft1, Some(env.user1.id()), NearToken::from_near(1))
        .await
        .unwrap();

    {
        let storage_balance_ft1_user1 = env
            .storage_balance_of(&env.ft1, env.user1.id())
            .await
            .unwrap();

        let storage_balance_ft1_user2 = env
            .storage_balance_of(&env.ft1, env.user2.id())
            .await
            .unwrap();

        assert_eq!(
            storage_balance_ft1_user1.unwrap().total,
            MIN_FT_STORAGE_DEPOSIT_VALUE
        );
        assert!(storage_balance_ft1_user2.is_none());
    }

    // The user should have enough wnear in his account (in his account in the wnear contract)
    env.user2
        .near_deposit(env.wnear.id(), NearToken::from_near(100))
        .await
        .unwrap();

    let nonce = rng.random();

    let signed_intents = [env.user2.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce,
        Deadline::timeout(std::time::Duration::from_secs(120)),
        DefuseIntents {
            intents: [StorageDeposit {
                contract_id: env.ft1.clone(),
                account_id: env.user2.id().clone(),
                amount: MIN_FT_STORAGE_DEPOSIT_VALUE,
            }
            .into()]
            .into(),
        },
    )];

    // Fails because the user does not own any wNEAR in the intents smart contract. They should first deposit wNEAR.
    env.defuse
        .execute_intents(signed_intents)
        .await
        .unwrap_err();
}
