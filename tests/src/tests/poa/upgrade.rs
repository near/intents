use std::collections::HashMap;

use defuse_sandbox::{
    account::Account,
    extensions::{
        acl::AccessControllableExt,
        poa::{PoAFactoryExt, PoaFactoryClient, PoaFactoryDeployerExt, contract::Role},
    },
    kit::{AccountId, Final, Near, NearToken},
    root,
};
use defuse_test_utils::wasms::{POA_FACTORY_LEGACY_WASM, POA_FACTORY_WASM};
use rstest::rstest;

/// Deploys `poa-factory` with the *legacy* (pre-omni-layer) wasm, using only
/// roles that already existed back then - the legacy `Role` enum doesn't
/// know about `OmniProver`, so including it in the init call would fail to
/// deserialize on-chain.
async fn deploy_legacy_factory(root: &Near) -> (Near, PoaFactoryClient) {
    root.deploy_poa_factory_with_account(
        "poa-factory",
        [root.account_id().clone()],
        [
            (Role::DAO, [root.account_id().clone()]),
            (Role::TokenDeployer, [root.account_id().clone()]),
            (Role::TokenDepositer, [root.account_id().clone()]),
        ],
        [
            (Role::DAO, [root.account_id().clone()]),
            (Role::TokenDeployer, [root.account_id().clone()]),
            (Role::TokenDepositer, [root.account_id().clone()]),
        ],
        POA_FACTORY_LEGACY_WASM.clone(),
    )
    .await
}

/// End-to-end test of the lazy state migration: deploys the *real* legacy
/// (pre-omni-layer) `poa-factory` wasm, creates real state through it
/// (a deployed token + a deposit), redeploys the *current* wasm on top
/// (no explicit `migrate()` call - it no longer exists), and verifies:
/// - the old state (token registry, balance) survived the upgrade;
/// - the new omni-layer functionality is usable immediately, without any
///   separate migration step.
#[rstest]
#[tokio::test]
async fn test_poa_factory_upgrade_from_legacy(#[future(awt)] root: Near) {
    let user = root
        .create_subaccount("user1", NearToken::from_near(10))
        .await;

    let (factory_account, poa_factory) = deploy_legacy_factory(&root).await;
    println!(
        "deployed LEGACY poa-factory at {} ({} bytes)",
        poa_factory.contract_id(),
        POA_FACTORY_LEGACY_WASM.len()
    );

    // Create real state through the OLD contract.
    let ft1 = root
        .poa_factory_deploy_token(poa_factory.contract_id(), "ft1", None)
        .await
        .unwrap();

    ft1.storage_deposit(user.account_id(), NearToken::from_near(1))
        .await
        .unwrap();

    root.poa_factory_ft_deposit(
        poa_factory.contract_id(),
        "ft1",
        user.account_id(),
        1_000,
        None,
        None,
    )
    .await
    .unwrap();

    // Record state "before" the upgrade.
    let tokens_before: HashMap<String, AccountId> = poa_factory.tokens().await.unwrap();
    let balance_before: u128 = ft1.balance_of(user.account_id()).await.unwrap().into();
    println!("BEFORE upgrade: tokens={tokens_before:?}, ft1 balance={balance_before}");
    assert_eq!(balance_before, 1_000);
    assert!(tokens_before.contains_key("ft1"));

    // Upgrade: just redeploy the current code, in place. No `migrate()` call -
    // it was removed; the lazy versioned (de)serialization takes over on the
    // very next read.
    factory_account
        .deploy(POA_FACTORY_WASM.clone())
        .wait_until::<Final>()
        .await
        .unwrap()
        .result()
        .unwrap();
    println!(
        "upgraded poa-factory at {} to the current wasm ({} bytes)",
        poa_factory.contract_id(),
        POA_FACTORY_WASM.len()
    );

    // Old state must still be there, unchanged, without any explicit migration step.
    let tokens_after: HashMap<String, AccountId> = poa_factory.tokens().await.unwrap();
    let balance_after: u128 = ft1.balance_of(user.account_id()).await.unwrap().into();
    println!("AFTER upgrade: tokens={tokens_after:?}, ft1 balance={balance_after}");
    assert_eq!(tokens_after, tokens_before);
    assert_eq!(balance_after, balance_before);

    // The legacy contract never knew about `OmniProver` - grant it now, on
    // the upgraded contract, using the already-existing super-admin.
    root.acl_grant_role(
        poa_factory.contract_id().clone(),
        "OmniProver",
        root.account_id(),
    )
    .await
    .unwrap();

    // New omni-layer functionality must work immediately after the upgrade,
    // with no separate migration call in between.
    let omni_tokens_after_upgrade = poa_factory.get_omni_tokens().await.unwrap();
    println!("omni_tokens right after upgrade: {omni_tokens_after_upgrade:?}");
    assert!(
        omni_tokens_after_upgrade.is_empty(),
        "omni_tokens must start out empty on an upgraded legacy contract"
    );

    root.poa_factory_add_omni_tokens(poa_factory.contract_id(), vec!["ft1".to_string()])
        .await
        .unwrap();
    let omni_tokens = poa_factory.get_omni_tokens().await.unwrap();
    println!("omni_tokens after add_omni_tokens: {omni_tokens:?}");
    assert_eq!(omni_tokens, vec!["ft1".to_string()]);

    // ft1 is now an omni token: the plain `ft_deposit` path must be rejected.
    let err = root
        .poa_factory_ft_deposit(
            poa_factory.contract_id(),
            "ft1",
            user.account_id(),
            1,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("omni token deposit requires omni_deposit method"),
        "unexpected error: {err:?}"
    );

    // The omni deposit path works and deduplicates by `deposit_id`.
    root.poa_factory_ft_omni_deposit(
        poa_factory.contract_id(),
        "deposit-1",
        "ft1",
        user.account_id(),
        500,
        None,
        None,
    )
    .await
    .unwrap();

    let err = root
        .poa_factory_ft_omni_deposit(
            poa_factory.contract_id(),
            "deposit-1",
            "ft1",
            user.account_id(),
            500,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("deposit already exists"),
        "unexpected error: {err:?}"
    );

    let balance_final: u128 = ft1.balance_of(user.account_id()).await.unwrap().into();
    println!("FINAL ft1 balance after omni deposit: {balance_final}");
    assert_eq!(balance_final, balance_before + 500);
}
