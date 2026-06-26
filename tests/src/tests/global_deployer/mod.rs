#[cfg(feature = "escrow-swap")]
mod deploy_escrow_swap;

use defuse_digest::{Digest, sha2::Sha256};
use defuse_sandbox::{
    account::Account,
    extensions::{
        FnCallTransaction,
        global_deployer::{
            GDApproveArgs, GDDeployerExt, GlobalDeployer, GlobalDeployerExt,
            contract::{
                AsWrap, Event, Reason, State as DeployerState,
                error::{ERR_NEW_CODE_HASH_MISMATCH, ERR_UNAUTHORIZED},
            },
        },
    },
    global_contract::GlobalContract,
    kit::{ExecutionStatus, Final, Gas, GlobalContractId, Near, NearToken},
    root,
};
use defuse_test_utils::{asserts::ResultAssertsExt, wasms::MT_RECEIVER_STUB_WASM};
use futures::future::join_all;
use near_sdk::AsNep297Event;
use rstest::{fixture, rstest};
use std::{
    future::IntoFuture,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use crate::utils::wasms::DEPLOYER_WASM;

static SUB_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct DeployerEnv {
    pub root: Near,
    pub deployer_global_id: GlobalContractId,
}

#[fixture]
pub async fn deployer_env(#[future(awt)] root: Near) -> DeployerEnv {
    let deployer_global_id = root
        .deploy_immutable_global_contract(
            root.account_id().sub_account("deployer").unwrap(),
            DEPLOYER_WASM.clone(),
            NearToken::from_near(20),
        )
        .await
        .unwrap();

    DeployerEnv {
        root,
        deployer_global_id,
    }
}

#[fixture]
pub fn unique_index() -> u32 {
    SUB_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[rstest]
#[tokio::test]
async fn test_deploy_controller_instance(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let alice = root
        .create_subaccount("alice", NearToken::from_near(1))
        .await;

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.account_id().clone()).with_index(unique_index);

    let upgradeable_instance_state = DeployerState::new(alice.account_id().clone());

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    root.deploy_gd_instance(
        GlobalContractId::AccountId(controller_instance.contract_id().clone()),
        upgradeable_instance_state.clone(),
    )
    .await
    .map(|_| ())
    .unwrap_err();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        state.code_hash,
    );

    root.gd_approve_and_deploy(
        controller_instance.contract_id(),
        state.code_hash,
        &DEPLOYER_WASM,
    )
    .await
    .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        Sha256::digest(&*DEPLOYER_WASM),
    );

    let mutable_controller_instance = root
        .deploy_gd_instance(
            GlobalContractId::AccountId(controller_instance.contract_id().clone()),
            upgradeable_instance_state.clone(),
        )
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_owner_id().await.unwrap(),
        *state.owner_id
    );
    assert_eq!(
        mutable_controller_instance.gd_owner_id().await.unwrap(),
        *upgradeable_instance_state.owner_id
    );

    assert_eq!(
        GlobalContractId::CodeHash(
            *root
                .account(controller_instance.contract_id())
                .await
                .unwrap()
                .global_contract_hash
                .unwrap()
                .as_bytes()
        ),
        deployer_code_hash_id
    );

    assert_eq!(
        &root
            .account(mutable_controller_instance.contract_id())
            .await
            .unwrap()
            .global_contract_account_id
            .unwrap(),
        controller_instance.contract_id()
    );
}

#[rstest]
#[tokio::test]
async fn test_refund_storage_deposit_when_its_not_enough_to_cover_storage_costs(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let initial_balance = NearToken::from_near(2);
    let owner = root.create_subaccount("dummy", initial_balance).await;

    assert_eq!(
        root.balance(owner.account_id()).await.unwrap().total,
        initial_balance
    );

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let storage = DeployerState::new(owner.account_id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        root.balance(controller_instance.contract_id())
            .await
            .unwrap()
            .total,
        NearToken::from_near(0)
    );

    owner
        .gd_approve(
            controller_instance.contract_id(),
            storage.code_hash,
            Sha256::digest(&*DEPLOYER_WASM),
        )
        .await
        .unwrap();

    let storage_deposit = NearToken::from_near(1);

    owner
        .fn_call(
            controller_instance.contract_id(),
            GlobalDeployer::gd_deploy(AsWrap::new(DEPLOYER_WASM.clone())),
            storage_deposit,
        )
        .await
        .assert_err_contains("NEAR to cover storage cost");

    let after = root.balance(owner.account_id()).await.unwrap().total;

    // NOTE: we expect the storage deposit to be refunded lets account for 10% less because
    // some balance is used to cover fees
    let min_expected_balance = storage_deposit.saturating_mul(10).saturating_div(9);

    assert!(
        after > min_expected_balance,
        "Storage deposit should be refunded (minus spent gas)"
    );
}
#[rstest]
#[tokio::test]
async fn test_transfer_ownership(#[future(awt)] deployer_env: DeployerEnv, unique_index: u32) {
    let root = deployer_env.root;
    let (alice, bob) = futures::future::join(
        root.create_subaccount("alice", NearToken::from_near(100)),
        root.create_subaccount("bob", NearToken::from_near(100)),
    )
    .await;

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let storage = DeployerState::new(alice.account_id().clone()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_owner_id().await.unwrap(),
        *storage.owner_id
    );

    // Non-owner cannot approve
    bob.gd_approve(
        controller_instance.contract_id(),
        storage.code_hash,
        Sha256::digest(&*DEPLOYER_WASM),
    )
    .await
    .assert_err_contains(ERR_UNAUTHORIZED);

    // Non-owner cannot transfer ownership
    bob.gd_transfer_ownership(controller_instance.contract_id(), bob.account_id())
        .await
        .assert_err_contains(ERR_UNAUTHORIZED);

    // Owner transfers ownership
    let result = alice
        .gd_transfer_ownership(controller_instance.contract_id(), bob.account_id())
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::Transfer {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
            }
            .to_nep297_event()
            .to_event_log(),
            Event::Approve {
                code_hash: DeployerState::DEFAULT_HASH,
                reason: Reason::By(bob.account_id().into()),
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );

    assert_eq!(
        controller_instance.gd_owner_id().await.unwrap(),
        bob.account_id().clone()
    );
}

#[rstest]
#[tokio::test]
async fn test_deploy_event_is_emitted(#[future(awt)] deployer_env: DeployerEnv, unique_index: u32) {
    let root = deployer_env.root;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(root.account_id().clone()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    root.gd_approve(
        controller_instance.contract_id(),
        storage.code_hash,
        Sha256::digest(&*DEPLOYER_WASM),
    )
    .await
    .unwrap();

    let result = root
        .gd_deploy(
            controller_instance.contract_id(),
            &DEPLOYER_WASM,
            NearToken::from_near(50),
        )
        .await
        .unwrap();

    let deployed_hash = Sha256::digest(&*DEPLOYER_WASM);
    assert_eq!(
        result.logs(),
        vec![
            Event::Deploy {
                code_hash: deployed_hash.into(),
            }
            .to_nep297_event()
            .to_event_log(),
            Event::Approve {
                code_hash: DeployerState::DEFAULT_HASH,
                reason: Reason::Deploy(deployed_hash.into()),
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        deployed_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_deploy_event_old_hash_after_upgrade(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(root.account_id().clone()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    // Step 1: Initial deploy of DEPLOYER_WASM
    root.gd_approve_and_deploy(
        controller_instance.contract_id(),
        storage.code_hash,
        &DEPLOYER_WASM,
    )
    .await
    .unwrap();

    let deployer_hash = Sha256::digest(&*DEPLOYER_WASM);
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        deployer_hash,
    );

    // Step 2: Approve + deploy upgrade to MT_RECEIVER_STUB_WASM
    let mt_stub_hash = Sha256::digest(&*MT_RECEIVER_STUB_WASM);
    let result = root
        .gd_approve_and_deploy(
            controller_instance.contract_id(),
            deployer_hash,
            &MT_RECEIVER_STUB_WASM,
        )
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::Approve {
                code_hash: mt_stub_hash.into(),
                reason: Reason::By(root.account_id().into()),
            }
            .to_nep297_event()
            .to_event_log(),
            Event::Deploy {
                code_hash: mt_stub_hash.into(),
            }
            .to_nep297_event()
            .to_event_log(),
            Event::Approve {
                code_hash: DeployerState::DEFAULT_HASH,
                reason: Reason::Deploy(mt_stub_hash.into()),
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        mt_stub_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_concurrent_upgrades_only_one_succeeds(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.account_id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // Initial deploy so controller has code
    root.gd_approve_and_deploy(
        controller_instance.contract_id(),
        state.code_hash,
        &DEPLOYER_WASM,
    )
    .await
    .unwrap();
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        Sha256::digest(&*DEPLOYER_WASM),
    );

    let old_hash = Sha256::digest(&*DEPLOYER_WASM);

    // Approve the upgrade before firing concurrent calls
    root.gd_approve(
        controller_instance.contract_id(),
        old_hash,
        Sha256::digest(&*MT_RECEIVER_STUB_WASM),
    )
    .await
    .unwrap();

    // Fire 10 concurrent upgrade calls all using the same old_hash
    let results = join_all((0..10).map(|_| {
        root.gd_deploy(
            controller_instance.contract_id(),
            &MT_RECEIVER_STUB_WASM,
            NearToken::from_near(50),
        )
        .into_future()
    }))
    .await;

    let successes = results.iter().filter(|r| r.is_ok()).count();
    let wrong_hash_failures = results
        .iter()
        .filter(|r| {
            r.as_ref()
                .is_err_and(|e| e.to_string().contains(ERR_NEW_CODE_HASH_MISMATCH))
        })
        .count();

    assert_eq!(
        successes, 1,
        "exactly one concurrent upgrade should succeed"
    );
    assert_eq!(wrong_hash_failures, 9);
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        Sha256::digest(&*MT_RECEIVER_STUB_WASM),
    );
}

#[rstest]
#[tokio::test]
async fn test_second_approval_overwrites_first(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.account_id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // First approval
    let first_hash = Sha256::digest(&*DEPLOYER_WASM);
    root.gd_approve(
        controller_instance.contract_id(),
        state.code_hash,
        first_hash,
    )
    .await
    .unwrap();

    // Second approval with different new_hash overwrites the first
    let second_hash = Sha256::digest(&*MT_RECEIVER_STUB_WASM);
    root.gd_approve(
        controller_instance.contract_id(),
        state.code_hash,
        second_hash,
    )
    .await
    .unwrap();

    // The second approval should be persisted
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        second_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_approve_revoke_resets_to_code_hash(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    // State starts with both code_hash and approved_hash set to [0; 32]
    let state = DeployerState::new(root.account_id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // Approve some arbitrary hash
    let arbitrary_hash = Sha256::digest(&*DEPLOYER_WASM);
    root.gd_approve(
        controller_instance.contract_id(),
        state.code_hash,
        arbitrary_hash,
    )
    .await
    .unwrap();
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        arbitrary_hash,
    );

    // Revoke the approval by resetting approved_hash back to code_hash ([0; 32]).
    // This is a valid use case: the owner changed their mind and wants to cancel
    // a previously approved deployment. Setting approved_hash equal to code_hash
    // effectively disables `gd_deploy` since new code can never hash to [0; 32].
    //
    // NOTE: this also proves that approving a hash already stored as code_hash is
    // allowed — the contract intentionally places no restriction on new_hash.
    root.gd_approve(
        controller_instance.contract_id(),
        state.code_hash,
        state.code_hash,
    )
    .await
    .unwrap();
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        state.code_hash, // back to [0; 32]
    );
}

#[rstest]
#[tokio::test]
async fn test_permissionless_deploy_with_approval(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let (alice, bob) = futures::future::join(
        root.create_subaccount("alice", NearToken::from_near(100)),
        root.create_subaccount("bob", NearToken::from_near(100)),
    )
    .await;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(alice.account_id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // Owner approves deployment
    let new_code_hash = Sha256::digest(&*DEPLOYER_WASM);
    alice
        .gd_approve(
            controller_instance.contract_id(),
            state.code_hash,
            new_code_hash,
        )
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        new_code_hash,
    );

    // Non-owner (bob) deploys successfully with matching approved_hash
    bob.gd_deploy(
        controller_instance.contract_id(),
        &DEPLOYER_WASM,
        NearToken::from_near(50),
    )
    .await
    .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        new_code_hash,
    );

    // approved_hash is reset after deploy
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        DeployerState::DEFAULT_HASH,
    );

    // Non-owner cannot deploy again without new approval
    bob.gd_deploy(
        controller_instance.contract_id(),
        &MT_RECEIVER_STUB_WASM,
        NearToken::from_near(50),
    )
    .await
    .assert_err_contains(ERR_NEW_CODE_HASH_MISMATCH);
}

#[rstest]
#[tokio::test]
async fn test_refund_excessive_deposit_attached_to_deploy(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let initial_balance = NearToken::from_near(200);
    let owner = root.create_implicit(initial_balance).await;

    assert_eq!(
        root.balance(owner.account_id()).await.unwrap().total,
        initial_balance
    );

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.account_id().clone()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        root.balance(controller_instance.contract_id())
            .await
            .unwrap()
            .total,
        NearToken::from_near(0)
    );

    owner
        .gd_approve(
            controller_instance.contract_id(),
            storage.code_hash,
            Sha256::digest(&*DEPLOYER_WASM),
        )
        .await
        .unwrap();

    owner
        .fn_call(
            controller_instance.contract_id(),
            GlobalDeployer::gd_deploy(AsWrap::new(DEPLOYER_WASM.clone())),
            NearToken::from_near(100), // attach more than enough to cover storage
        )
        .await
        .unwrap();

    assert!(
        root.balance(controller_instance.contract_id())
            .await
            .unwrap()
            .total
            < NearToken::from_millinear(900)
    );
}

#[rstest]
#[tokio::test]
async fn test_state_init_pre_approve_allows_immediate_deploy(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let bob = root
        .create_subaccount("bob", NearToken::from_near(100))
        .await;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    // Pre-set approved_hash so gd_deploy can be called immediately without gd_approve
    let state = DeployerState::new(root.account_id().clone())
        .with_index(unique_index)
        .pre_approve(Sha256::digest(&*DEPLOYER_WASM));

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    assert!(bob.account_id().clone() != controller_instance.gd_owner_id().await.unwrap());
    bob.gd_deploy(
        controller_instance.contract_id(),
        &DEPLOYER_WASM,
        NearToken::from_near(50),
    )
    .await
    .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        Sha256::digest(&*DEPLOYER_WASM),
    );

    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        DeployerState::DEFAULT_HASH,
    );
}

#[rstest]
#[tokio::test]
async fn test_state_init_same_code_hash_and_pre_approve_allows_deploy(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let dummy_wasm: Vec<u8> = vec![1u8; 64];
    let dummy_hash = Sha256::digest(&dummy_wasm);

    // State where code_hash == approved_hash == hash(dummy_wasm)
    let mut state = DeployerState::new(root.account_id().clone())
        .with_index(unique_index)
        .pre_approve(dummy_hash);
    state.code_hash = dummy_hash.into();

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        dummy_hash
    );
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        dummy_hash,
    );

    // Deploy should succeed even though code_hash == approved_hash
    root.gd_deploy(
        controller_instance.contract_id(),
        &dummy_wasm,
        NearToken::from_near(50),
    )
    .await
    .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        dummy_hash,
    );

    // Approval should be cleared after deploy
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        DeployerState::DEFAULT_HASH,
    );
}

#[rstest]
#[tokio::test]
async fn test_post_deploy_does_not_run_on_failed_deploy(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let initial_balance = NearToken::from_near(2);
    let owner = root.create_subaccount("dummy2", initial_balance).await;

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.account_id().clone()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    // Instance has 0 balance
    assert_eq!(
        root.balance(controller_instance.contract_id())
            .await
            .unwrap()
            .total,
        NearToken::from_near(0)
    );

    let new_hash = Sha256::digest(&*DEPLOYER_WASM);

    owner
        .gd_approve(
            controller_instance.contract_id(),
            storage.code_hash,
            new_hash,
        )
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        new_hash,
    );

    // Deploy with insufficient deposit → LackBalanceForState
    owner
        .gd_deploy(
            controller_instance.contract_id(),
            &DEPLOYER_WASM,
            NearToken::from_yoctonear(1), // not enough to cover state
        )
        .await
        .assert_err_contains("NEAR to cover storage cost");

    // code_hash must NOT have been updated by gd_post_deploy
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        storage.code_hash,
    );

    // approved_hash must NOT have been cleared by gd_post_deploy
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        new_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_retry_approve_and_deploy_after_insufficient_deposit(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let owner = root
        .create_subaccount("retry", NearToken::from_near(100))
        .await;

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.account_id().clone()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    let new_hash = Sha256::digest(&*DEPLOYER_WASM);

    // First attempt: gd_approve_and_deploy with insufficient deposit (1 NEAR)
    owner
        .transaction(controller_instance.contract_id())
        .add_action(
            GlobalDeployer::gd_approve(GDApproveArgs {
                old_hash: storage.code_hash,
                new_hash: new_hash.into(),
            })
            .deposit(NearToken::from_yoctonear(1))
            .gas(Gas::from_tgas(10)),
        )
        .add_action(
            GlobalDeployer::gd_deploy(AsWrap::new(DEPLOYER_WASM.clone()))
                .deposit(NearToken::from_near(1))
                .gas(Gas::from_tgas(290)),
        )
        .await
        .unwrap()
        .result()
        .assert_err_contains("NEAR to cover storage cost");

    // code_hash unchanged after failed deploy
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        storage.code_hash,
    );

    // Retry with sufficient deposit
    owner
        .gd_approve_and_deploy(
            controller_instance.contract_id(),
            storage.code_hash,
            &DEPLOYER_WASM,
        )
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        new_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_post_deploy_fails_when_approval_changed(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.account_id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // Initial deploy so controller has real code
    root.gd_approve_and_deploy(
        controller_instance.contract_id(),
        state.code_hash,
        &DEPLOYER_WASM,
    )
    .await
    .unwrap();

    let deployer_hash = Sha256::digest(&*DEPLOYER_WASM);
    let mt_stub_hash = Sha256::digest(&*MT_RECEIVER_STUB_WASM);
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        deployer_hash,
    );

    // Approve re-deploying the same DEPLOYER_WASM code. This way, after the deploy
    // promise succeeds, the controller still runs the deployer code and we can query state.
    root.gd_approve(
        controller_instance.contract_id(),
        deployer_hash,
        deployer_hash,
    )
    .await
    .unwrap();

    // Batch transaction:
    //   Action 1: gd_deploy(DEPLOYER_WASM) — passes approved_hash check, creates deploy+callback
    //   Action 2: gd_approve(deployer_hash, mt_stub_hash) — changes approved_hash to mt_stub_hash
    //
    // In NEAR, batch actions execute sequentially in the same receipt, but promise receipts
    // from action 1 execute in later blocks — so action 2's state change is visible to the
    // callback. The callback sees approved_hash == mt_stub_hash != deployer_hash and panics.
    let result = root
        .transaction(controller_instance.contract_id())
        .add_action(
            GlobalDeployer::gd_deploy(AsWrap::new(DEPLOYER_WASM.clone()))
                .deposit(NearToken::from_near(50))
                .gas(Gas::from_tgas(140)),
        )
        .add_action(
            GlobalDeployer::gd_approve(GDApproveArgs {
                old_hash: deployer_hash.into(),
                new_hash: mt_stub_hash.into(),
            })
            .deposit(NearToken::from_yoctonear(1))
            .gas(Gas::from_tgas(10)),
        )
        .wait_until(Final)
        .await
        .unwrap();

    // The callback (gd_post_deploy) should have failed because approved_hash was changed
    // by the second action in the batch before the callback executed
    assert!(
        result
            .receipts_outcome
            .iter()
            .any(|o| matches!(o.outcome.status, ExecutionStatus::Failure(_))),
        "gd_post_deploy callback should have failed"
    );

    result.result().unwrap();

    // code_hash should be unchanged — the callback was rejected so state was not updated
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        deployer_hash,
    );

    // approved_hash should be mt_stub_hash (set by gd_approve in action 2)
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap().0,
        mt_stub_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_deploy_with_zero_deposit_and_prefunded_account(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let owner = root
        .create_subaccount("prefund", NearToken::from_near(100))
        .await;

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.account_id().clone()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        root.balance(controller_instance.contract_id())
            .await
            .unwrap()
            .total,
        NearToken::from_near(0)
    );

    // Pre-fund the deterministic account so it has enough for storage
    root.transaction(controller_instance.contract_id())
        .transfer(NearToken::from_near(50))
        .send()
        .wait_until(Final)
        .await
        .unwrap()
        .result()
        .unwrap();

    assert_eq!(
        root.balance(controller_instance.contract_id())
            .await
            .unwrap()
            .total,
        NearToken::from_near(50)
    );

    owner
        .gd_approve(
            controller_instance.contract_id(),
            storage.code_hash,
            Sha256::digest(&*DEPLOYER_WASM),
        )
        .await
        .unwrap();

    owner
        .gd_deploy(
            controller_instance.contract_id(),
            &DEPLOYER_WASM,
            NearToken::from_yoctonear(0),
        )
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        Sha256::digest(&*DEPLOYER_WASM),
    );
}

#[rstest]
#[tokio::test]
async fn test_concurrent_transfer_does_not_inflate_refund(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let initial_balance = NearToken::from_near(200);
    let owner = root.create_implicit(initial_balance).await;

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.account_id().clone()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    owner
        .gd_approve_and_deploy(
            controller_instance.contract_id(),
            storage.code_hash,
            &DEPLOYER_WASM,
        )
        .await
        .unwrap();
    let deployer_hash = Sha256::digest(&*DEPLOYER_WASM);
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        deployer_hash,
    );

    let mt_stub_hash = Sha256::digest(&*MT_RECEIVER_STUB_WASM);
    owner
        .gd_approve(
            controller_instance.contract_id(),
            deployer_hash,
            mt_stub_hash,
        )
        .await
        .unwrap();

    // Create 50 accounts that will each transfer 4 NEAR (200 NEAR total)
    let num_senders = 50u128;
    let transfer_amount = NearToken::from_near(4);
    let senders: Vec<Near> =
        join_all((0..num_senders).map(|_| root.create_implicit(NearToken::from_near(10)))).await;

    let owner_balance_before_deploy = root.balance(owner.account_id()).await.unwrap().total;

    let deploy_deposit = NearToken::from_near(50);

    let deploy_handle = tokio::spawn({
        let controller_id = controller_instance.contract_id().clone();
        let owner = owner.clone();
        async move {
            owner
                .gd_deploy(controller_id, &MT_RECEIVER_STUB_WASM, deploy_deposit)
                .await
                .unwrap()
        }
    });
    let transfer_futs = senders.iter().map(|s| {
        s.transaction(controller_instance.contract_id())
            .transfer(transfer_amount)
            .into_future()
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    join_all(transfer_futs).await;

    deploy_handle.await.unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        mt_stub_hash,
    );

    let owner_balance_after = root.balance(owner.account_id()).await.unwrap().total;
    // Some transfers land between gd_deploy and gd_post_deploy, inflating
    // account_balance well above initial_balance + attached_deposit. The refund
    // cap `min(excess, attached_deposit)` limits refund to 50 NEAR
    assert!(
        owner_balance_after <= owner_balance_before_deploy,
        "owner balance should not increase after deploy"
    );

    let deploy_price = NearToken::from_micronear(100); // 0001 NEAR per 1B
    let contract_size = MT_RECEIVER_STUB_WASM.len().try_into().unwrap();
    let deploy_cost = deploy_price.checked_mul(contract_size).unwrap();

    let owner_spent = owner_balance_before_deploy.saturating_sub(owner_balance_after);
    let contract_balance_after = root
        .balance(controller_instance.contract_id())
        .await
        .unwrap()
        .total;
    let transferred_amount = transfer_amount.checked_mul(num_senders).unwrap();

    // There can be 2 options:
    // 1) If transfer amount equal or greater to deploy cost land before the post deploy,
    // then owner gets a full refund and spends near only on gas
    // 2) Otherwise, owner only gets a partial refund

    let refund = deploy_deposit
        .saturating_sub(deploy_cost)
        .saturating_add(transferred_amount.saturating_sub(contract_balance_after));

    let max_gas = NearToken::from_near(1);
    let expected_outlay = deploy_deposit
        .saturating_sub(refund)
        .saturating_add(max_gas);

    assert!(owner_spent <= expected_outlay);
}

#[rstest]
#[tokio::test]
async fn test_gd_deploy_accepts_raw_bytes(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.root;
    let owner = root.create_implicit(NearToken::from_near(200)).await;
    let storage = DeployerState::new(owner.account_id().clone()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_env.deployer_global_id.clone(), storage.clone())
        .await
        .unwrap();

    owner
        .gd_approve(
            controller_instance.contract_id(),
            storage.code_hash,
            Sha256::digest(&*DEPLOYER_WASM),
        )
        .await
        .unwrap();

    owner
        .gd_deploy(
            controller_instance.contract_id(),
            &DEPLOYER_WASM,
            NearToken::from_near(50),
        )
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        Sha256::digest(&*DEPLOYER_WASM),
    );
}
