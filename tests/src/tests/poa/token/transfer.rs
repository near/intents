use super::token_env::{PoATokenContract, PoATokenContractCaller, PoATokenExt};
use crate::{
    tests::poa::token::token_env::MIN_FT_STORAGE_DEPOSIT_VALUE,
    utils::{Sandbox, ft::FtExt, storage_management::StorageManagementExt, wnear::WNearExt},
};
use defuse_poa_token::WITHDRAW_MEMO_PREFIX;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::{AccountId, NearToken};
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
    async fn near_balance(&self, account_id: &AccountId) -> NearToken {
        self.sandbox
            .worker()
            .view_account(account_id)
            .await
            .unwrap()
            .balance
    }

    async fn new() -> Self {
        let sandbox = Sandbox::new().await.unwrap();
        let root = sandbox.root_account().clone();
        let poa_contract_owner = sandbox.create_account("owner").await;
        let user1 = sandbox.create_account("user1").await;
        let user2 = sandbox.create_account("user2").await;
        let poa_token_contract: PoATokenContract = root
            .deploy_poa_token("poa_token", Some(poa_contract_owner.id()), None)
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

/// Tests ft_transfer, ft_deposit, balances and withdrawals with and without wrapping
#[tokio::test]
async fn simple_transfer() {
    let fixture = TransferFixture::new().await;

    // fund user1 with deposit
    {
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user1.id())
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
                .poa_ft_balance_of(fixture.user1.id())
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
                .poa_ft_balance_of(fixture.user2.id())
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
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            40_000.into()
        );

        assert!(!logs.logs().iter().any(|s| s.contains("ft_burn")));
    }

    // Burning tokens by using the special case and transferring to the smart contract address
    {
        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            40_000.into()
        );

        let total_supply_before_burn = fixture
            .poa_token_contract
            .poa_ft_total_supply()
            .await
            .unwrap();

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

        // Assert that a burn event was emitted
        assert!(logs.logs().iter().any(|s| s.contains("ft_burn")));
        assert!(logs.logs().iter().any(|s| {
            s.replace(' ', "")
                .contains(&"\"amount\":\"10000\"".to_string())
        }));

        let total_supply_after_burn = fixture
            .poa_token_contract
            .poa_ft_total_supply()
            .await
            .unwrap();

        // Supply went down by the burned amount
        assert_eq!(
            total_supply_after_burn.0 + 10000,
            total_supply_before_burn.0
        );

        assert_eq!(
            fixture
                .poa_token_contract
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            30_000.into()
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
                .poa_ft_balance_of(fixture.user2.id())
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
                .poa_ft_balance_of(fixture.user2.id())
                .await
                .unwrap(),
            35_000.into()
        );

        assert!(!logs.logs().iter().any(|s| s.contains("ft_burn")));
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

    // Deposit after wrapping should fail
    {
        assert!(
            fixture
                .poa_contract_owner
                .poa_ft_deposit(
                    &fixture.poa_token_contract,
                    fixture.user1.id(),
                    10_000.into(),
                    None,
                )
                .await
                .unwrap_err()
                .to_string()
                .contains("This PoA token was migrated to OmniBridge. No deposits are possible")
        );
    }
}

#[tokio::test]
async fn metadata_sync() {
    let fixture = TransferFixture::new().await;

    // Unauthorized user attempts syncing
    assert!(
        fixture
            .user1
            .poa_force_sync_wrapped_token_metadata(
                &fixture.poa_token_contract,
                NearToken::from_near(1)
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("Method is private")
    );

    // Cannot sync metadata before wrapping
    assert!(
        fixture
            .poa_contract_owner
            .poa_force_sync_wrapped_token_metadata(
                &fixture.poa_token_contract,
                NearToken::from_near(1)
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("This function is restricted to wrapped tokens")
    );

    // Deploy wrapped near
    let wnear_contract = fixture.sandbox.deploy_wrap_near("wnear").await.unwrap();

    // Wrap the PoA token
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

    // Attempting to update metadata but with only 1 yocto, which is not enough for storage deposit, because we're adding more info
    assert!(
        fixture
            .poa_contract_owner
            .poa_force_sync_wrapped_token_metadata(
                &fixture.poa_token_contract,
                NearToken::from_yoctonear(1)
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("Not enough attached deposit for updating metadata")
    );

    let balance_before = fixture.near_balance(fixture.poa_token_contract.id()).await;

    // syncing metadata should work now with enough sufficient deposit
    fixture
        .poa_contract_owner
        .poa_force_sync_wrapped_token_metadata(&fixture.poa_token_contract, NearToken::from_near(1))
        .await
        .unwrap();

    let balance_after = fixture.near_balance(fixture.poa_token_contract.id()).await;

    // Updating the metadata shouldn't consume more than 10 millinear
    assert!(
        balance_after
            > balance_before
                .checked_sub(NearToken::from_millinear(10))
                .unwrap()
    );

    // Check the metadata against the wrapped token
    let source_metadata: FungibleTokenMetadata = wnear_contract
        .call("ft_metadata")
        .view()
        .await
        .unwrap()
        .json()
        .unwrap();

    let new_metadata = fixture.poa_token_contract.poa_ft_metadata().await.unwrap();
    assert_eq!(new_metadata.symbol, format!("w{}", source_metadata.symbol));
    assert_eq!(
        new_metadata.name,
        format!("Wrapped {}", source_metadata.name),
    );

    // Attempting to redo synchronization is OK
    fixture
        .poa_contract_owner
        .poa_force_sync_wrapped_token_metadata(
            &fixture.poa_token_contract,
            NearToken::from_yoctonear(1),
        ) // only 1 yocto because we're not changing anything
        .await
        .unwrap();
}
