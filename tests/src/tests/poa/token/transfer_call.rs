use super::token_env::{PoATokenContract, PoATokenContractCaller, PoATokenExt};
use crate::utils::Sandbox;
use defuse_poa_token::UNWRAP_PREFIX;
use near_workspaces::Account;
use rstest::rstest;
use test_utils::random::{Seed, make_random_string, make_seedable_rng, random_seed};

struct TransferCallFixture {
    #[allow(dead_code)]
    sandbox: Sandbox,
    #[allow(dead_code)]
    root: Account,
    user1: Account,
    user2: Account,
    poa_contract_owner: Account,
    poa_token_contract: PoATokenContract,
    second_poa_contract_owner: Account,
    second_poa_token_contract: PoATokenContract,
    third_poa_contract_owner: Account,
    third_poa_token_contract: PoATokenContract,
}

impl TransferCallFixture {
    async fn new() -> Self {
        let sandbox = Sandbox::new().await.unwrap();
        let root = sandbox.root_account().clone();
        let user1 = sandbox.create_account("user1").await;
        let user2 = sandbox.create_account("user2").await;
        let poa_contract_owner = sandbox.create_account("owner").await;
        let poa_token_contract: PoATokenContract = root
            .deploy_poa_token("poa_token", Some(poa_contract_owner.id().clone()), None)
            .await
            .unwrap();

        let second_poa_contract_owner = sandbox.create_account("owner2").await;
        let second_poa_token_contract: PoATokenContract = root
            .deploy_poa_token(
                "poa_token2",
                Some(second_poa_contract_owner.id().clone()),
                None,
            )
            .await
            .unwrap();

        let third_poa_contract_owner = sandbox.create_account("owner3").await;
        let third_poa_token_contract: PoATokenContract = root
            .deploy_poa_token(
                "poa_token3",
                Some(third_poa_contract_owner.id().clone()),
                None,
            )
            .await
            .unwrap();

        // Storage deposit for involved users, to deposit tokens into his account
        {
            root.poa_storage_deposit_simple(&poa_token_contract, user1.id())
                .await
                .unwrap();
            root.poa_storage_deposit_simple(&poa_token_contract, user2.id())
                .await
                .unwrap();
            root.poa_storage_deposit_simple(&poa_token_contract, second_poa_token_contract.id())
                .await
                .unwrap();

            root.poa_storage_deposit_simple(&second_poa_token_contract, user1.id())
                .await
                .unwrap();
            root.poa_storage_deposit_simple(&second_poa_token_contract, user2.id())
                .await
                .unwrap();
            root.poa_storage_deposit_simple(
                &second_poa_token_contract,
                third_poa_token_contract.id(),
            )
            .await
            .unwrap();

            root.poa_storage_deposit_simple(&third_poa_token_contract, user1.id())
                .await
                .unwrap();
            root.poa_storage_deposit_simple(&third_poa_token_contract, user2.id())
                .await
                .unwrap();
        }

        Self {
            sandbox,
            root,
            user1,
            user2,
            poa_contract_owner,
            poa_token_contract,
            second_poa_contract_owner,
            second_poa_token_contract,
            third_poa_contract_owner,
            third_poa_token_contract,
        }
    }
}

#[tokio::test]
#[rstest]
#[trace]
async fn transfer_and_call(random_seed: Seed) {
    let mut rng = make_seedable_rng(random_seed);

    let fixture = TransferCallFixture::new().await;

    // fund user1 with deposit
    {
        fixture
            .poa_contract_owner
            .poa_ft_deposit(
                &fixture.poa_token_contract,
                fixture.user1.id(),
                100_000.into(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            100_000.into()
        );
    }

    // Make the second PoA token a wrap of the first one
    {
        // No token wraps in PoA so far
        assert!(
            fixture
                .second_poa_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .is_none()
        );

        fixture
            .second_poa_contract_owner
            .poa_set_wrapped_token_account_id(
                &fixture.second_poa_token_contract,
                fixture.poa_token_contract.id(),
            )
            .await
            .unwrap();

        assert_eq!(
            fixture
                .second_poa_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .as_ref(),
            Some(fixture.poa_token_contract.id())
        );
    }

    // Make the third PoA token a wrap of the second one
    {
        // No token wraps in PoA so far
        assert!(
            fixture
                .third_poa_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .is_none()
        );

        fixture
            .third_poa_contract_owner
            .poa_set_wrapped_token_account_id(
                &fixture.third_poa_token_contract,
                fixture.second_poa_token_contract.id(),
            )
            .await
            .unwrap();

        assert_eq!(
            fixture
                .third_poa_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .as_ref(),
            Some(fixture.second_poa_token_contract.id())
        );
    }

    // Testing ft_on_transfer
    // Transferring to another account/contract (on first poa token contract, which is unwrapped) does a simple ft_transfer_call in the inner token
    // `msg` is empty. The sender should receive the balance (based on ft_on_transfer in the second contract).
    {
        // Balance before
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.second_poa_token_contract.id())
                .await
                .unwrap(),
            0.into()
        );

        // Transfer
        fixture
            .user1
            .poa_ft_transfer_call(
                &fixture.poa_token_contract,
                fixture.second_poa_token_contract.id(),
                10_000.into(),
                None,
                String::new(),
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.second_poa_token_contract.id())
                .await
                .unwrap(),
            10_000.into()
        );
    }

    // Testing ft_on_transfer
    // Transferring to another account/contract (on first poa token contract, which is unwrapped) to second contract, does a simple ft_transfer_call in the inner token
    // `msg` has user2 id. They should receive that balance in the second contract (based on ft_on_transfer in the second contract).
    {
        // Balance before (second contract's balance in first's)
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.second_poa_token_contract.id())
                .await
                .unwrap(),
            10_000.into()
        );

        // Balance before (user2 balance in second contract)
        assert_eq!(
            fixture
                .second_poa_token_contract
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            0.into()
        );

        // Transfer
        fixture
            .user1
            .poa_ft_transfer_call(
                &fixture.poa_token_contract,
                fixture.second_poa_token_contract.id(),
                5_000.into(),
                None,
                fixture.user2.id().to_string(),
            )
            .await
            .unwrap();

        // Balance after (second contract's balance in first's)
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.second_poa_token_contract.id())
                .await
                .unwrap(),
            15_000.into()
        );

        // Balance after (user2 balance in second contract)
        assert_eq!(
            fixture
                .second_poa_token_contract
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            5_000.into()
        );
    }

    // Testing ft_transfer_call
    // On a contract with a wrapped token (second contract), if the receiver is NOT the contract account id, it will still use the inner token's transfer function
    // which will call ft_on_transfer on the third poa token contract with the same message, giving the funds to user2, the sender, because it's an empty message
    {
        // Balance before
        assert_eq!(
            fixture
                .third_poa_token_contract
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            0.into()
        );

        // Transfer
        fixture
            .user2
            .poa_ft_transfer_call(
                &fixture.second_poa_token_contract,
                fixture.third_poa_token_contract.id(),
                200.into(),
                None,
                String::new(),
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .third_poa_token_contract
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            200.into()
        );
    }

    // Testing ft_transfer_call
    // On a contract with a wrapped token (second contract), if the receiver is NOT the contract account id, it will still use the inner token's transfer function
    // which will call ft_on_transfer on the third poa token contract with the same message, giving the funds to user1, because user1 is specified there
    {
        // Balance before
        assert_eq!(
            fixture
                .third_poa_token_contract
                .poa_ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            0.into()
        );

        // Transfer
        fixture
            .user2
            .poa_ft_transfer_call(
                &fixture.second_poa_token_contract,
                fixture.third_poa_token_contract.id(),
                300.into(),
                None,
                fixture.user1.id().to_string(),
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .third_poa_token_contract
                .poa_ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            300.into()
        );
    }

    // Testing ft_transfer_call
    // Using a random message will lead to a NO-OP
    {
        let msg = make_random_string(&mut rng, 30);
        // Balance before
        assert_eq!(
            fixture
                .second_poa_token_contract
                .poa_ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            10000.into()
        );

        assert_eq!(
            fixture
                .second_poa_token_contract
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            4500.into()
        );

        // Transfer
        fixture
            .user2
            .poa_ft_transfer_call(
                &fixture.second_poa_token_contract,
                fixture.user1.id(),
                500.into(),
                None,
                msg,
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .second_poa_token_contract
                .poa_ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            10000.into()
        );

        assert_eq!(
            fixture
                .second_poa_token_contract
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            4500.into()
        );
    }

    // Testing ft_transfer_call
    // Using the contract's address as destination + a message with the unwrap prefix + an invalid address will panic
    {
        // Transfer
        fixture
            .user2
            .poa_ft_transfer_call(
                &fixture.second_poa_token_contract,
                fixture.second_poa_token_contract.id(),
                500.into(),
                None,
                format!("{UNWRAP_PREFIX}:HELLO_WORLD"),
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("Invalid account id provided in msg");
    }
}
