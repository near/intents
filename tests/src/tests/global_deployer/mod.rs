#[cfg(feature = "escrow-swap")]
mod deploy_escrow_swap;

use std::sync::atomic::{AtomicU32, Ordering};

use defuse_global_deployer::{
    Event, State as DeployerState,
    error::{ERR_NEW_CODE_HASH_MISMATCH, ERR_UNAUTHORIZED, ERR_WRONG_CODE_HASH},
};
use defuse_sandbox::{
    Sandbox,
    api::types::transaction::actions::GlobalContractDeployMode,
    extensions::global_deployer::{DeployerExt, DeployerViewExt},
    sandbox,
    tx::FnCallBuilder,
};
use defuse_test_utils::{asserts::ResultAssertsExt, wasms::MT_RECEIVER_STUB_WASM};
use futures::future::join_all;
use near_sdk::{AsNep297Event, GlobalContractId, NearToken, env::sha256_array};
use rstest::{fixture, rstest};

use crate::utils::wasms::DEPLOYER_WASM;

static SUB_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct DeployerEnv {
    pub sandbox: Sandbox,
    pub deployer_global_id: GlobalContractId,
}

#[fixture]
pub async fn deployer_env() -> DeployerEnv {
    let sandbox = sandbox(NearToken::from_near(100_000)).await;
    let root = sandbox.root();

    root.deploy_global_contract(DEPLOYER_WASM.clone(), GlobalContractDeployMode::CodeHash)
        .await
        .unwrap();
    DeployerEnv {
        sandbox,
        deployer_global_id: GlobalContractId::CodeHash(sha256_array(&*DEPLOYER_WASM).into()),
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
    let root = deployer_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(1))
        .await
        .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.id().clone(), unique_index);

    let upgradeable_instance_state = DeployerState::new(alice.id().clone(), 0);

    let controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    root.deploy_instance(
        GlobalContractId::AccountId(controller_instance.id().clone()),
        upgradeable_instance_state.clone(),
    )
    .await
    .assert_err_contains("GlobalContractDoesNotExist");

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        DeployerState::DEFAULT_HASH,
    );

    root.gd_approve_and_deploy(
        controller_instance.id(),
        DeployerState::DEFAULT_HASH,
        &DEPLOYER_WASM,
    )
    .await
    .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let mutable_controller_instance = root
        .deploy_instance(
            GlobalContractId::AccountId(controller_instance.id().clone()),
            upgradeable_instance_state.clone(),
        )
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_owner_id().await.unwrap(),
        state.owner_id
    );
    assert_eq!(controller_instance.gd_index().await.unwrap(), state.index);
    assert_eq!(
        mutable_controller_instance.gd_owner_id().await.unwrap(),
        upgradeable_instance_state.owner_id
    );
    assert_eq!(
        mutable_controller_instance.gd_index().await.unwrap(),
        upgradeable_instance_state.index
    );

    assert_eq!(
        controller_instance.global_contract_id().await.unwrap(),
        deployer_code_hash_id
    );

    assert_eq!(
        mutable_controller_instance
            .global_contract_id()
            .await
            .unwrap(),
        GlobalContractId::AccountId(controller_instance.id().clone())
    );
}

#[rstest]
#[tokio::test]
async fn test_refund_storage_deposit_when_its_not_enough_to_cover_storage_costs(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox.root();
    let initial_balance = NearToken::from_near(2);
    let owner = root
        .generate_subaccount("dummy", initial_balance)
        .await
        .unwrap();

    assert_eq!(owner.view().await.unwrap().amount, initial_balance);

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let storage = DeployerState::new(owner.id().clone(), unique_index);
    let controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        controller_instance.view().await.unwrap().amount,
        NearToken::from_near(0)
    );

    owner
        .gd_approve(
            controller_instance.id(),
            DeployerState::DEFAULT_HASH,
            sha256_array(&*DEPLOYER_WASM),
        )
        .await
        .unwrap();

    let storage_deposit = NearToken::from_near(1);
    owner
        .tx(controller_instance.id())
        .function_call(
            FnCallBuilder::new("gd_deploy")
                .borsh_args(&(DeployerState::DEFAULT_HASH, &*DEPLOYER_WASM))
                .with_deposit(storage_deposit),
        )
        .await
        .assert_err_contains("LackBalanceForState");

    let after = owner.view().await.unwrap().amount;

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
    let root = deployer_env.sandbox.root();
    let (alice, bob) = futures::future::try_join(
        root.generate_subaccount("alice", NearToken::from_near(100)),
        root.generate_subaccount("bob", NearToken::from_near(100)),
    )
    .await
    .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let storage = DeployerState::new(alice.id().clone(), unique_index);

    let controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_owner_id().await.unwrap(),
        storage.owner_id
    );
    assert_eq!(controller_instance.gd_index().await.unwrap(), storage.index);

    // Non-owner cannot approve
    bob.gd_approve(
        controller_instance.id(),
        DeployerState::DEFAULT_HASH,
        sha256_array(&*DEPLOYER_WASM),
    )
    .await
    .assert_err_contains(ERR_UNAUTHORIZED);

    // Non-owner cannot transfer ownership
    bob.gd_transfer_ownership(controller_instance.id(), bob.id())
        .await
        .assert_err_contains(ERR_UNAUTHORIZED);

    // Owner transfers ownership
    let result = alice
        .gd_transfer_ownership(controller_instance.id(), bob.id())
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::Transfer {
                old_owner_id: alice.id().into(),
                new_owner_id: bob.id().into(),
            }
            .to_nep297_event()
            .to_event_log()
        ]
    );

    assert_eq!(
        controller_instance.gd_owner_id().await.unwrap(),
        bob.id().clone()
    );
}

#[rstest]
#[tokio::test]
async fn test_deploy_event_is_emitted(#[future(awt)] deployer_env: DeployerEnv, unique_index: u32) {
    let root = deployer_env.sandbox.root();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(root.id().clone(), unique_index);

    let controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    root.gd_approve(
        controller_instance.id(),
        DeployerState::DEFAULT_HASH,
        sha256_array(&*DEPLOYER_WASM),
    )
    .await
    .unwrap();

    let result = root
        .gd_deploy(
            controller_instance.id(),
            DeployerState::DEFAULT_HASH,
            &DEPLOYER_WASM,
        )
        .await
        .unwrap();

    let expected_event = defuse_global_deployer::Event::Deploy {
        old_hash: DeployerState::DEFAULT_HASH,
        new_hash: sha256_array(&*DEPLOYER_WASM),
    };
    assert!(
        result
            .logs()
            .contains(&expected_event.to_nep297_event().to_event_log().as_str())
    );
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );
}

#[rstest]
#[tokio::test]
async fn test_concurrent_upgrades_only_one_succeeds(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox.root();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let controller_instance = root
        .deploy_instance(
            deployer_code_hash_id.clone(),
            DeployerState::new(root.id().clone(), unique_index),
        )
        .await
        .unwrap();

    // Initial deploy so controller has code
    root.gd_approve_and_deploy(
        controller_instance.id(),
        DeployerState::DEFAULT_HASH,
        &DEPLOYER_WASM,
    )
    .await
    .unwrap();
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let old_hash = sha256_array(&*DEPLOYER_WASM);

    // Approve the upgrade before firing concurrent calls
    root.gd_approve(
        controller_instance.id(),
        old_hash,
        sha256_array(&*MT_RECEIVER_STUB_WASM),
    )
    .await
    .unwrap();

    // Fire 10 concurrent upgrade calls all using the same old_hash
    let results = join_all(
        (0..10).map(|_| root.gd_deploy(controller_instance.id(), old_hash, &MT_RECEIVER_STUB_WASM)),
    )
    .await;

    let successes = results.iter().filter(|r| r.is_ok()).count();
    let wrong_hash_failures = results
        .iter()
        .filter(|r| {
            r.as_ref()
                .is_err_and(|e| e.to_string().contains(ERR_WRONG_CODE_HASH))
        })
        .count();

    assert_eq!(
        successes, 1,
        "exactly one concurrent upgrade should succeed"
    );
    assert_eq!(
        wrong_hash_failures, 9,
        "remaining 9 should fail with wrong code hash"
    );
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*MT_RECEIVER_STUB_WASM),
    );
}

#[rstest]
#[tokio::test]
async fn test_second_approval_overwrites_first(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox.root();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let controller_instance = root
        .deploy_instance(
            deployer_code_hash_id.clone(),
            DeployerState::new(root.id().clone(), unique_index),
        )
        .await
        .unwrap();

    // First approval
    let first_hash = sha256_array(&*DEPLOYER_WASM);
    root.gd_approve(
        controller_instance.id(),
        DeployerState::DEFAULT_HASH,
        first_hash,
    )
    .await
    .unwrap();

    // Second approval with different new_hash overwrites the first
    let second_hash = sha256_array(&*MT_RECEIVER_STUB_WASM);
    root.gd_approve(
        controller_instance.id(),
        DeployerState::DEFAULT_HASH,
        second_hash,
    )
    .await
    .unwrap();

    // The second approval should be persisted
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap(),
        second_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_permissionless_deploy_with_approval(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox.root();
    let (alice, bob) = futures::future::try_join(
        root.generate_subaccount("alice", NearToken::from_near(100)),
        root.generate_subaccount("bob", NearToken::from_near(100)),
    )
    .await
    .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let controller_instance = root
        .deploy_instance(
            deployer_code_hash_id.clone(),
            DeployerState::new(alice.id().clone(), unique_index),
        )
        .await
        .unwrap();

    // Owner approves deployment
    let new_code_hash = sha256_array(&*DEPLOYER_WASM);
    alice
        .gd_approve(
            controller_instance.id(),
            DeployerState::DEFAULT_HASH,
            new_code_hash,
        )
        .await
        .unwrap();

    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap(),
        new_code_hash,
    );

    // Non-owner (bob) deploys successfully with matching approved_hash
    bob.gd_deploy(
        controller_instance.id(),
        DeployerState::DEFAULT_HASH,
        &DEPLOYER_WASM,
    )
    .await
    .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        new_code_hash,
    );

    // approved_hash is reset after deploy
    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap(),
        DeployerState::DEFAULT_HASH,
    );

    // Non-owner cannot deploy again without new approval
    bob.gd_deploy(
        controller_instance.id(),
        new_code_hash,
        &MT_RECEIVER_STUB_WASM,
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
    let root = deployer_env.sandbox.root();
    let initial_balance = NearToken::from_near(200);
    let owner = root.fund_implicit(initial_balance).await.unwrap();

    assert_eq!(owner.view().await.unwrap().amount, initial_balance);
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.id().clone(), unique_index);

    let controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        controller_instance.view().await.unwrap().amount,
        NearToken::from_near(0)
    );

    owner
        .gd_approve(
            controller_instance.id(),
            DeployerState::DEFAULT_HASH,
            sha256_array(&*DEPLOYER_WASM),
        )
        .await
        .unwrap();

    owner
        .tx(controller_instance.id())
        .function_call(
            FnCallBuilder::new("gd_deploy")
                .borsh_args(&(DeployerState::DEFAULT_HASH, &*DEPLOYER_WASM))
                .with_deposit(NearToken::from_near(100)),
        )
        .await
        .unwrap();

    let controller_instance_balance = controller_instance.view().await.unwrap().amount;
    assert!(controller_instance_balance < NearToken::from_millinear(900));
}

#[rstest]
#[tokio::test]
async fn test_state_init_with_approved_hash_allows_immediate_deploy(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox.root();
    let bob = root
        .generate_subaccount("bob", NearToken::from_near(100))
        .await
        .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    // Pre-set approved_hash so gd_deploy can be called immediately without gd_approve
    let state = DeployerState {
        owner_id: root.id().clone(),
        index: unique_index,
        code_hash: DeployerState::DEFAULT_HASH,
        approved_hash: sha256_array(&*DEPLOYER_WASM),
    };

    let controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), state)
        .await
        .unwrap();

    bob.gd_deploy(
        controller_instance.id(),
        DeployerState::DEFAULT_HASH,
        &DEPLOYER_WASM,
    )
    .await
    .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    assert_eq!(
        controller_instance.gd_approved_hash().await.unwrap(),
        DeployerState::DEFAULT_HASH,
    );
}

