use super::token_env::PoATokenContractCaller;
use crate::{tests::poa::token_env::PoATokenExt, utils::Sandbox};
use defuse_poa_token::WITHDRAW_MEMO_PREFIX;
use randomness::Rng;
use rstest::rstest;
use test_utils::random::{Seed, make_seedable_rng, random_seed};

#[tokio::test]
#[rstest]
#[trace]
async fn simple_transfer(random_seed: Seed) {
    let mut rng = make_seedable_rng(random_seed);

    let sandbox = Sandbox::new().await.unwrap();
    let root = sandbox.root_account();
    let contract_owner = sandbox.create_account("owner").await;
    let user1 = sandbox.create_account("user1").await;
    let user2 = sandbox.create_account("user2").await;
    let poa_token_contract = root
        .deploy_poa_token("poa_token", Some(contract_owner.id().clone()), None)
        .await
        .unwrap();

    let user1_balance = rng.random_range::<u128, _>(100..100_000);

    // Storage deposit for involved users, to deposit tokens into his account
    {
        root.storage_deposit_simple(&poa_token_contract, user1.id())
            .await
            .unwrap();
        root.storage_deposit_simple(&poa_token_contract, user2.id())
            .await
            .unwrap();
    }

    // fund user1 with deposit
    {
        assert_eq!(
            poa_token_contract
                .ft_balance_of(user1.id().clone())
                .await
                .unwrap(),
            0.into()
        );

        contract_owner
            .ft_deposit(&poa_token_contract, user1.id(), user1_balance.into(), None)
            .await
            .unwrap();

        assert_eq!(
            poa_token_contract
                .ft_balance_of(user1.id().clone())
                .await
                .unwrap(),
            user1_balance.into()
        );
    }

    let user1_to_2_transfer_amount: u128 = rng.random_range(1..user1_balance);
    // transfer from user1 to user2
    {
        assert_eq!(
            poa_token_contract
                .ft_balance_of(user2.id().clone())
                .await
                .unwrap(),
            0.into()
        );

        let logs = user1
            .ft_transfer(
                &poa_token_contract,
                user2.id(),
                user1_to_2_transfer_amount.into(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            poa_token_contract
                .ft_balance_of(user2.id().clone())
                .await
                .unwrap(),
            user1_to_2_transfer_amount.into()
        );

        assert!(!logs.as_strings().iter().any(|s| s.contains("ft_burn")));
    }

    let user2_to_burn = rng.random_range(1..=user1_to_2_transfer_amount);
    // Burning tokens by using the special case and transferring to the smart contract address
    {
        assert_eq!(
            poa_token_contract
                .ft_balance_of(user2.id().clone())
                .await
                .unwrap(),
            user1_to_2_transfer_amount.into()
        );

        let logs = user2
            .ft_transfer(
                &poa_token_contract,
                poa_token_contract.id(),
                user2_to_burn.into(),
                Some(WITHDRAW_MEMO_PREFIX.to_owned()),
            )
            .await
            .unwrap();

        // Assert that a burn has happened through the logs
        assert!(logs.as_strings().iter().any(|s| s.contains("ft_burn")));
        assert!(logs.as_strings().iter().any(|s| {
            s.replace(' ', "")
                .contains(&format!("\"amount\":\"{user2_to_burn}\""))
        }));

        assert_eq!(
            poa_token_contract
                .ft_balance_of(user2.id().clone())
                .await
                .unwrap(),
            (user1_to_2_transfer_amount - user2_to_burn).into()
        );
    }
}
