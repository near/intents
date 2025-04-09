use super::token_env::{
    MIN_FT_STORAGE_DEPOSIT_VALUE, PoATokenContract, PoATokenContractCaller, PoATokenExt,
};
use crate::utils::{Sandbox, ft::FtExt, storage_management::StorageManagementExt};
use defuse_poa_token::UNWRAP_PREFIX;
use near_sdk::NearToken;
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

    // l1 -> Level 1 -> Doesn't wrap anything
    // L1
    poa_l1_contract_owner: Account,
    poa_l1_token_contract: PoATokenContract,

    // l2 -> Level 2 -> Wraps level1
    // L2
    poa_l2_contract_owner: Account,
    poa_l2_token_contract: PoATokenContract,

    // l3 -> Level 3 -> Wraps level2
    // L3
    poa_l3_contract_owner: Account,
    poa_l3_token_contract: PoATokenContract,
}

impl TransferCallFixture {
    async fn new() -> Self {
        let sandbox = Sandbox::new().await.unwrap();
        let root = sandbox.root_account().clone();
        let user1 = sandbox.create_account("user1").await;
        let user2 = sandbox.create_account("user2").await;
        let poa_l1_contract_owner = sandbox.create_account("owner").await;
        let poa_l1_token_contract: PoATokenContract = root
            .deploy_poa_token("poa_token", Some(poa_l1_contract_owner.id()), None)
            .await
            .unwrap();

        let poa_l2_contract_owner = sandbox.create_account("owner2_1").await;
        let poa_l2_token_contract: PoATokenContract = root
            .deploy_poa_token("poa_token2_1", Some(poa_l2_contract_owner.id()), None)
            .await
            .unwrap();

        let poa_l3_contract_owner = sandbox.create_account("owner3").await;
        let poa_l3_token_contract: PoATokenContract = root
            .deploy_poa_token("poa_token3", Some(poa_l3_contract_owner.id()), None)
            .await
            .unwrap();

        // Storage deposit for involved users, to deposit tokens into his account
        {
            root.storage_deposit(
                poa_l1_token_contract.id(),
                Some(user1.id()),
                MIN_FT_STORAGE_DEPOSIT_VALUE,
            )
            .await
            .unwrap();
            root.storage_deposit(
                poa_l1_token_contract.id(),
                Some(user2.id()),
                MIN_FT_STORAGE_DEPOSIT_VALUE,
            )
            .await
            .unwrap();
            root.storage_deposit(
                poa_l1_token_contract.id(),
                Some(poa_l2_token_contract.id()),
                MIN_FT_STORAGE_DEPOSIT_VALUE,
            )
            .await
            .unwrap();

            root.storage_deposit(
                poa_l2_token_contract.id(),
                Some(user1.id()),
                MIN_FT_STORAGE_DEPOSIT_VALUE,
            )
            .await
            .unwrap();
            root.storage_deposit(
                poa_l2_token_contract.id(),
                Some(user2.id()),
                MIN_FT_STORAGE_DEPOSIT_VALUE,
            )
            .await
            .unwrap();
            root.storage_deposit(
                poa_l2_token_contract.id(),
                Some(poa_l3_token_contract.id()),
                MIN_FT_STORAGE_DEPOSIT_VALUE,
            )
            .await
            .unwrap();

            root.storage_deposit(
                poa_l3_token_contract.id(),
                Some(user1.id()),
                MIN_FT_STORAGE_DEPOSIT_VALUE,
            )
            .await
            .unwrap();
            root.storage_deposit(
                poa_l3_token_contract.id(),
                Some(user2.id()),
                MIN_FT_STORAGE_DEPOSIT_VALUE,
            )
            .await
            .unwrap();
        }

        Self {
            sandbox,
            root,
            user1,
            user2,
            poa_l1_contract_owner,
            poa_l1_token_contract,
            poa_l2_contract_owner,
            poa_l2_token_contract,
            poa_l3_contract_owner,
            poa_l3_token_contract,
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
            .poa_l1_contract_owner
            .poa_ft_deposit(
                &fixture.poa_l1_token_contract,
                fixture.user1.id(),
                100_000.into(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            fixture
                .poa_l1_token_contract
                .inner()
                .ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            100_000
        );
    }

    // Make the L2 PoA token a wrap of the L1 contract
    {
        // No token wraps in PoA so far
        assert!(
            fixture
                .poa_l2_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .is_none()
        );

        {
            fixture
                .poa_l2_contract_owner
                .poa_lock_contract_for_wrapping(fixture.poa_l2_token_contract.id())
                .await
                .unwrap();

            fixture
                .poa_l2_contract_owner
                .poa_set_wrapped_token_account_id(
                    &fixture.poa_l2_token_contract,
                    fixture.poa_l1_token_contract.id(),
                    NearToken::from_near(1),
                )
                .await
                .unwrap();

            fixture
                .poa_l2_contract_owner
                .poa_unlock_contract_for_wrapping(fixture.poa_l2_token_contract.id())
                .await
                .unwrap();
        }

        assert_eq!(
            fixture
                .poa_l2_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .as_ref(),
            Some(fixture.poa_l1_token_contract.id())
        );
    }

    // Make the L3 PoA token a wrap of L2
    {
        // No token wraps in PoA so far
        assert!(
            fixture
                .poa_l3_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .is_none()
        );

        {
            assert!(
                !fixture
                    .poa_l3_token_contract
                    .poa_is_contract_locked_for_wrapping()
                    .await
                    .unwrap()
            );

            fixture
                .poa_l3_contract_owner
                .poa_lock_contract_for_wrapping(fixture.poa_l3_token_contract.id())
                .await
                .unwrap();

            assert!(
                fixture
                    .poa_l3_token_contract
                    .poa_is_contract_locked_for_wrapping()
                    .await
                    .unwrap()
            );

            fixture
                .poa_l3_contract_owner
                .poa_set_wrapped_token_account_id(
                    &fixture.poa_l3_token_contract,
                    fixture.poa_l2_token_contract.id(),
                    NearToken::from_near(1),
                )
                .await
                .unwrap();

            fixture
                .poa_l3_contract_owner
                .poa_unlock_contract_for_wrapping(fixture.poa_l3_token_contract.id())
                .await
                .unwrap();

            assert!(
                !fixture
                    .poa_l3_token_contract
                    .poa_is_contract_locked_for_wrapping()
                    .await
                    .unwrap()
            );
        }

        assert_eq!(
            fixture
                .poa_l3_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .as_ref(),
            Some(fixture.poa_l2_token_contract.id())
        );
    }

    // Testing ft_on_transfer
    // Transferring to another account/contract (on L1 poa token contract, which is unwrapped) does a simple ft_transfer_call in the inner token
    // `msg` is empty. The sender should receive the balance (based on ft_on_transfer in the L2 contract).
    {
        // Balance before
        assert_eq!(
            fixture
                .poa_l1_token_contract
                .inner()
                .ft_balance_of(fixture.poa_l2_token_contract.id())
                .await
                .unwrap(),
            0
        );

        // Transfer
        fixture
            .user1
            .ft_transfer_call(
                fixture.poa_l1_token_contract.id(),
                fixture.poa_l2_token_contract.id(),
                10_000,
                None,
                "",
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .poa_l1_token_contract
                .inner()
                .ft_balance_of(fixture.poa_l2_token_contract.id())
                .await
                .unwrap(),
            10_000
        );
    }

    // Testing ft_on_transfer
    // Transferring to another account/contract (on L1 poa token contract, which is unwrapped) to L2 contract, does a simple ft_transfer_call in the inner token
    // `msg` has user2 id. They should receive that balance in the L2 contract (based on ft_on_transfer in the L2 contract).
    {
        // Balance before (L2 contract's balance in L1's)
        assert_eq!(
            fixture
                .poa_l1_token_contract
                .inner()
                .ft_balance_of(fixture.poa_l2_token_contract.id())
                .await
                .unwrap(),
            10_000
        );

        // Balance before (user2 balance in L2 contract)
        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            0
        );

        // Transfer
        fixture
            .user1
            .ft_transfer_call(
                fixture.poa_l1_token_contract.id(),
                fixture.poa_l2_token_contract.id(),
                5_000,
                None,
                fixture.user2.id().as_ref(),
            )
            .await
            .unwrap();

        // Balance after (L2 contract's balance in L1's)
        assert_eq!(
            fixture
                .poa_l1_token_contract
                .inner()
                .ft_balance_of(fixture.poa_l2_token_contract.id())
                .await
                .unwrap(),
            15_000
        );

        // Balance after (user2 balance in L2 contract)
        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            5_000
        );
    }

    // Testing ft_transfer_call
    // On a contract with a wrapped token (L2 contract), if the receiver is NOT the contract account id, it will still use the inner token's transfer function
    // which will call ft_on_transfer on the L3 poa token contract with the same message, giving the funds to user2, the sender, because it's an empty message
    {
        // Balance before
        assert_eq!(
            fixture
                .poa_l3_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            0
        );

        // Transfer
        fixture
            .user2
            .ft_transfer_call(
                fixture.poa_l2_token_contract.id(),
                fixture.poa_l3_token_contract.id(),
                200,
                None,
                "",
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .poa_l3_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            200
        );
    }

    // Testing ft_transfer_call
    // On a contract with a wrapped token (L2 contract), if the receiver is NOT the contract account id, it will still use the inner token's transfer function
    // which will call ft_on_transfer on the L3 poa token contract with the same message, giving the funds to user1, because user1 is specified there
    {
        // Balance before
        assert_eq!(
            fixture
                .poa_l3_token_contract
                .inner()
                .ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            0
        );

        // Transfer
        fixture
            .user2
            .ft_transfer_call(
                fixture.poa_l2_token_contract.id(),
                fixture.poa_l3_token_contract.id(),
                300,
                None,
                fixture.user1.id().as_ref(),
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .poa_l3_token_contract
                .inner()
                .ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            300
        );
    }

    // Testing ft_transfer_call
    // Using a random message will lead to a NO-OP
    {
        let msg = make_random_string(&mut rng, 30);
        // Balance before
        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            10000
        );

        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            4500
        );

        // Transfer
        fixture
            .user2
            .ft_transfer_call(
                fixture.poa_l2_token_contract.id(),
                fixture.user1.id(),
                500,
                None,
                &msg,
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            10000
        );

        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            4500
        );
    }

    // Testing ft_transfer_call
    // Using the contract's address as destination + a message with the unwrap prefix + an invalid address will panic
    {
        // Transfer
        fixture
            .user2
            .ft_transfer_call(
                fixture.poa_l2_token_contract.id(),
                fixture.poa_l2_token_contract.id(),
                500,
                None,
                &format!("{UNWRAP_PREFIX}HELLO_WORLD"),
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("Invalid account id provided in msg");
    }

    // Testing ft_transfer_call
    // Using the contract's address as destination + a message with the unwrap prefix + a valid address in the form UNWRAP_TO:receiver.near
    {
        let msg = format!("{UNWRAP_PREFIX}{}", fixture.user2.id());
        // Balance before
        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            10000
        );

        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            4500
        );

        // user2 balance in L1 contract
        assert_eq!(
            fixture
                .poa_l1_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            0
        );

        // Transfer
        fixture
            .user1
            .ft_transfer_call(
                fixture.poa_l2_token_contract.id(),
                fixture.poa_l2_token_contract.id(),
                500,
                None,
                &msg,
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            9500
        );

        assert_eq!(
            fixture
                .poa_l2_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            4500
        );

        // Balance of user2 in L1's contract
        assert_eq!(
            fixture
                .poa_l1_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            500
        );
    }

    // Testing ft_transfer_call
    // Using the contract's address as destination + a message with the unwrap prefix + a valid address in the form UNWRAP_TO:L3.near:UNWRAP_TO:user2
    // Call is done by user1.
    // This will unwrap from L3 into L2, which in turn will unwrap into L1 with a simple ft_transfer
    {
        let msg = format!(
            "{UNWRAP_PREFIX}{}:{UNWRAP_PREFIX}{}",
            fixture.poa_l2_token_contract.id(),
            fixture.user2.id()
        );
        // Balance before
        assert_eq!(
            fixture
                .poa_l3_token_contract
                .inner()
                .ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            300
        );

        // Balance in L1, which will be receiving the tokens after unwrapping twice
        assert_eq!(
            fixture
                .poa_l1_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            500
        );

        // Transfer
        fixture
            .user1
            .ft_transfer_call(
                fixture.poa_l3_token_contract.id(),
                fixture.poa_l3_token_contract.id(),
                120,
                None,
                &msg,
            )
            .await
            .unwrap();

        // Balance after
        assert_eq!(
            fixture
                .poa_l3_token_contract
                .inner()
                .ft_balance_of(fixture.user1.id())
                .await
                .unwrap(),
            180
        );

        // Balance of user1 in L1 contract, which we unwrapped to
        assert_eq!(
            fixture
                .poa_l1_token_contract
                .inner()
                .ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            620
        );
    }
}
