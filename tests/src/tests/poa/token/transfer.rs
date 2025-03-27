use super::token_env::{PoATokenContract, PoATokenContractCaller, PoATokenExt};
use crate::{
    tests::poa::token::token_env::MIN_FT_STORAGE_DEPOSIT_VALUE,
    utils::{Sandbox, ft::FtExt, storage_management::StorageManagementExt, wnear::WNearExt},
};
use defuse_poa_token::WITHDRAW_MEMO_PREFIX;
use near_sdk::NearToken;
use near_workspaces::Account;

struct TransferFixture {
    sandbox: Sandbox,
    poa_contract_owner: Account,
    root: Account,
    user1: Account,
    user2: Account,
    poa_token_contract: PoATokenContract,
}

impl TransferFixture {
    async fn new() -> Self {
        let sandbox = Sandbox::new().await.unwrap();
        let root = sandbox.root_account().clone();
        let poa_contract_owner = sandbox.create_account("owner").await;
        let user1 = sandbox.create_account("user1").await;
        let user2 = sandbox.create_account("user2").await;
        let poa_token_contract: PoATokenContract = root
            .deploy_poa_token("poa_token", Some(poa_contract_owner.id().clone()), None)
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
        }

        Self {
            sandbox,
            poa_contract_owner,
            root,
            user1,
            user2,
            poa_token_contract,
        }
    }
}

#[tokio::test]
async fn simple_transfer_after_wrap() {
    let fixture = TransferFixture::new().await;

    // fund user1 with deposit
    {
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user1.id().clone())
                .await
                .unwrap(),
            0.into()
        );

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
                .poa_ft_balance_of(fixture.user1.id().clone())
                .await
                .unwrap(),
            100_000.into()
        );
    }

    // transfer from user1 to user2
    {
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user2.id().clone())
                .await
                .unwrap(),
            0.into()
        );

        let logs = fixture
            .user1
            .poa_ft_transfer(
                &fixture.poa_token_contract,
                fixture.user2.id(),
                40_000.into(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user2.id().clone())
                .await
                .unwrap(),
            40_000.into()
        );

        assert!(!logs.as_strings().iter().any(|s| s.contains("ft_burn")));
    }

    // Burning tokens by using the special case and transferring to the smart contract address
    {
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user2.id().clone())
                .await
                .unwrap(),
            40_000.into()
        );

        let logs = fixture
            .user2
            .poa_ft_transfer(
                &fixture.poa_token_contract,
                fixture.poa_token_contract.id(),
                10_000.into(),
                Some(WITHDRAW_MEMO_PREFIX.to_owned()),
            )
            .await
            .unwrap();

        // Assert that a burn has happened through the logs
        assert!(logs.as_strings().iter().any(|s| s.contains("ft_burn")));
        assert!(logs.as_strings().iter().any(|s| {
            s.replace(' ', "")
                .contains(&"\"amount\":\"10000\"".to_string())
        }));

        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user2.id().clone())
                .await
                .unwrap(),
            (30_000).into()
        );
    }

    // Deploy wrapped near
    let wnear_contract = fixture.sandbox.deploy_wrap_near("wnear").await.unwrap();

    {
        // No token wraps in PoA so far
        assert!(
            fixture
                .poa_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .is_none()
        );

        // Attempt to deploy with the a non-owner
        assert!(
            fixture
                .user1
                .poa_set_wrapped_token_account_id(&fixture.poa_token_contract, wnear_contract.id())
                .await
                .unwrap_err()
                .to_string()
                .contains("Method is private")
        );

        // This will fail because the target contract we're wrapping, wnear, has no balance for the PoA contract.
        assert!(
            fixture
                .poa_contract_owner
                .poa_set_wrapped_token_account_id(&fixture.poa_token_contract, wnear_contract.id())
                .await
                .unwrap_err()
                .to_string()
                .contains("sufficient balance to cover")
        );

        // Fund wnear
        fixture
            .root
            .near_deposit(wnear_contract.id(), NearToken::from_near(10))
            .await
            .unwrap();

        fixture
            .root
            .storage_deposit(
                wnear_contract.id(),
                Some(fixture.poa_token_contract.id()),
                MIN_FT_STORAGE_DEPOSIT_VALUE,
            )
            .await
            .unwrap();

        fixture
            .root
            .ft_transfer(
                wnear_contract.id(),
                fixture.poa_token_contract.id(),
                100_000,
                None,
            )
            .await
            .unwrap();

        fixture
            .poa_contract_owner
            .poa_set_wrapped_token_account_id(&fixture.poa_token_contract, wnear_contract.id())
            .await
            .unwrap();

        assert_eq!(
            fixture
                .poa_token_contract
                .poa_wrapped_token()
                .await
                .unwrap()
                .as_ref(),
            Some(wnear_contract.id())
        );
    }

    // transfer from user1 to user2 should still work, even though it's wrapped
    {
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user2.id().clone())
                .await
                .unwrap(),
            30_000.into()
        );

        let logs = fixture
            .user1
            .poa_ft_transfer(
                &fixture.poa_token_contract,
                fixture.user2.id(),
                5_000.into(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user2.id().clone())
                .await
                .unwrap(),
            35_000.into()
        );

        assert!(!logs.as_strings().iter().any(|s| s.contains("ft_burn")));
    }

    // Burning tokens by using the special case and transferring to the smart contract address
    {
        assert!(
            fixture
                .user2
                .poa_ft_transfer(
                    &fixture.poa_token_contract,
                    fixture.poa_token_contract.id(),
                    10_000.into(),
                    Some(WITHDRAW_MEMO_PREFIX.to_owned()),
                )
                .await
                .unwrap_err()
                .to_string()
                .contains("PoA token was migrated to OmniBridge")
        );
    }
}
