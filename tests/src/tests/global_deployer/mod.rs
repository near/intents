use std::borrow::Cow;
use std::future::IntoFuture;
use std::sync::atomic::{AtomicU32, Ordering};

use defuse_global_deployer::AsHex;
use defuse_global_deployer::{
    Event, Reason, State as DeployerState,
    error::{ERR_NEW_CODE_HASH_MISMATCH, ERR_UNAUTHORIZED},
};
use defuse_sandbox::IntoAccountId;
use defuse_sandbox::extensions::global_deployer::{
    GDApproveArgs, GDDeployArgs, GDTransferOwnershipArgs,
};
use defuse_sandbox::near_kit::{
    FinalExecutionOutcome, Finality, GlobalContractIdentifier, TxExecutionStatus,
};
use defuse_sandbox::{Sandbox, sandbox};
use defuse_test_utils::{asserts::ResultAssertsExt, wasms::MT_RECEIVER_STUB_WASM};
use futures::future::{join_all, try_join};
use impl_tools::autoimpl;
use near_sdk::{AsNep297Event, Gas, NearToken, env::sha256_array};
use rstest::{fixture, rstest};

use crate::utils::wasms::DEPLOYER_WASM;

static SUB_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Collect all logs from all receipt outcomes.
fn all_logs(outcome: &FinalExecutionOutcome) -> Vec<String> {
    outcome
        .receipts_outcome
        .iter()
        .flat_map(|r| r.outcome.logs.iter().cloned())
        .collect()
}

#[autoimpl(Deref using self.sandbox)]
pub struct DeployerEnv {
    pub sandbox: Sandbox,
    pub deployer_global_id: GlobalContractIdentifier,
}

#[fixture]
pub async fn deployer_env(#[future(awt)] sandbox: Sandbox) -> DeployerEnv {
    sandbox
        .deploy_global_contract_by_hash(DEPLOYER_WASM.clone())
        .await
        .unwrap();

    DeployerEnv {
        sandbox,
        deployer_global_id: GlobalContractIdentifier::CodeHash(
            sha256_array(&*DEPLOYER_WASM).into(),
        ),
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
    let root = deployer_env.sandbox;
    let alice = root
        .generate_sub_account("alice", NearToken::from_near(1))
        .await
        .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.into_account_id()).with_index(unique_index);
    let upgradeable_instance_state = DeployerState::new(alice.into_account_id());

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // Deploying with AccountId reference fails before the global contract has code
    assert!(
        root.deploy_gd_instance(
            GlobalContractIdentifier::AccountId(controller_instance.contract_id().clone()),
            upgradeable_instance_state.clone(),
        )
        .await
        .is_err()
    );

    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        state.code_hash,
    );

    root.gd_approve_and_deploy(&controller_instance, state.code_hash, &DEPLOYER_WASM)
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        sha256_array(&*DEPLOYER_WASM),
    );

    let mutable_controller_instance = root
        .deploy_gd_instance(
            GlobalContractIdentifier::AccountId(controller_instance.contract_id().clone()),
            upgradeable_instance_state.clone(),
        )
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_owner_id()
            .finality(Finality::Optimistic)
            .await
            .unwrap(),
        state.owner_id
    );
    assert_eq!(
        mutable_controller_instance
            .gd_owner_id()
            .finality(Finality::Optimistic)
            .await
            .unwrap(),
        upgradeable_instance_state.owner_id
    );

    assert_eq!(
        root.global_contract_id(controller_instance.contract_id())
            .await
            .unwrap(),
        deployer_code_hash_id
    );

    assert_eq!(
        root.global_contract_id(mutable_controller_instance.contract_id())
            .await
            .unwrap(),
        GlobalContractIdentifier::AccountId(controller_instance.contract_id().clone())
    );
}

#[rstest]
#[tokio::test]
async fn test_refund_storage_deposit_when_its_not_enough_to_cover_storage_costs(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let initial_balance = NearToken::from_near(2);
    let owner = root
        .generate_sub_account("dummy", initial_balance)
        .await
        .unwrap();

    assert_eq!(
        owner.account(owner.into_account_id()).await.unwrap().amount,
        initial_balance
    );

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let storage = DeployerState::new(owner.into_account_id()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        root.account(controller_instance.contract_id())
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .amount,
        NearToken::from_near(0)
    );

    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(storage.code_hash),
            new_hash: AsHex(sha256_array(&*DEPLOYER_WASM)),
        })
        .deposit(NearToken::from_yoctonear(1))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap();

    let storage_deposit = NearToken::from_near(1);
    controller_instance
        .gd_deploy(GDDeployArgs {
            code: DEPLOYER_WASM.to_vec(),
        })
        .deposit(storage_deposit)
        .gas(Gas::from_tgas(300))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap_err();

    let after = owner
        .account(owner.into_account_id())
        .finality(Finality::Optimistic)
        .await
        .unwrap()
        .amount;

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
    let root = deployer_env.sandbox;
    let (alice, bob) = try_join(
        root.generate_sub_account("alice", NearToken::from_near(10)),
        root.generate_sub_account("bob", NearToken::from_near(10)),
    )
    .await
    .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let storage = DeployerState::new(alice.into_account_id()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_owner_id()
            .finality(Finality::Optimistic)
            .await
            .unwrap(),
        storage.owner_id
    );

    // Non-owner cannot approve
    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(storage.code_hash),
            new_hash: AsHex(sha256_array(&*DEPLOYER_WASM)),
        })
        .deposit(NearToken::from_yoctonear(1))
        .sign_with(bob.signer().unwrap())
        .await
        .assert_err_contains(ERR_UNAUTHORIZED);

    // Non-owner cannot transfer ownership
    controller_instance
        .gd_transfer_ownership(GDTransferOwnershipArgs {
            receiver_id: bob.into_account_id(),
        })
        .deposit(NearToken::from_yoctonear(1))
        .sign_with(bob.signer().unwrap())
        .await
        .assert_err_contains(ERR_UNAUTHORIZED);

    // Owner transfers ownership
    let alice_id = alice.into_account_id();
    let bob_id = bob.into_account_id();
    let result = controller_instance
        .gd_transfer_ownership(GDTransferOwnershipArgs {
            receiver_id: bob_id.clone(),
        })
        .deposit(NearToken::from_yoctonear(1))
        .sign_with(alice.signer().unwrap())
        .await
        .unwrap();

    assert_eq!(
        all_logs(&result),
        vec![
            Event::Transfer {
                old_owner_id: Cow::Borrowed(&*alice_id),
                new_owner_id: Cow::Borrowed(&*bob_id),
            }
            .to_nep297_event()
            .to_event_log(),
            Event::Approve {
                code_hash: DeployerState::DEFAULT_HASH,
                reason: Reason::By(Cow::Borrowed(&*bob_id)),
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );

    assert_eq!(
        controller_instance
            .gd_owner_id()
            .finality(Finality::Optimistic)
            .await
            .unwrap(),
        bob_id,
    );
}

#[rstest]
#[tokio::test]
async fn test_deploy_event_is_emitted(#[future(awt)] deployer_env: DeployerEnv, unique_index: u32) {
    let root = deployer_env.sandbox;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(root.into_account_id()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(storage.code_hash),
            new_hash: AsHex(sha256_array(&*DEPLOYER_WASM)),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .await
        .unwrap();

    let result = controller_instance
        .gd_deploy(GDDeployArgs {
            code: DEPLOYER_WASM.to_vec(),
        })
        .deposit(NearToken::from_near(50))
        .gas(Gas::from_tgas(290))
        .await
        .unwrap();

    let deployed_hash = sha256_array(&*DEPLOYER_WASM);
    assert_eq!(
        all_logs(&result),
        vec![
            Event::Deploy {
                code_hash: deployed_hash,
            }
            .to_nep297_event()
            .to_event_log(),
            Event::Approve {
                code_hash: DeployerState::DEFAULT_HASH,
                reason: Reason::Deploy(deployed_hash),
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );
    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        deployed_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_deploy_event_old_hash_after_upgrade(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(root.into_account_id()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    // Step 1: Initial deploy of DEPLOYER_WASM
    root.gd_approve_and_deploy(&controller_instance, storage.code_hash, &DEPLOYER_WASM)
        .await
        .unwrap();

    let deployer_hash = sha256_array(&*DEPLOYER_WASM);
    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        deployer_hash,
    );

    // Step 2: Approve + deploy upgrade to MT_RECEIVER_STUB_WASM
    let mt_stub_hash = sha256_array(&*MT_RECEIVER_STUB_WASM);

    let approve_result = controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(deployer_hash),
            new_hash: AsHex(mt_stub_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .await
        .unwrap();

    let deploy_result = controller_instance
        .gd_deploy(GDDeployArgs {
            code: MT_RECEIVER_STUB_WASM.to_vec(),
        })
        .deposit(NearToken::from_near(50))
        .gas(Gas::from_tgas(290))
        .await
        .unwrap();

    let root_id = root.into_account_id();
    let combined_logs: Vec<String> = all_logs(&approve_result)
        .into_iter()
        .chain(all_logs(&deploy_result))
        .collect();
    assert_eq!(
        combined_logs,
        vec![
            Event::Approve {
                code_hash: mt_stub_hash,
                reason: Reason::By(Cow::Borrowed(&*root_id)),
            }
            .to_nep297_event()
            .to_event_log(),
            Event::Deploy {
                code_hash: mt_stub_hash,
            }
            .to_nep297_event()
            .to_event_log(),
            Event::Approve {
                code_hash: DeployerState::DEFAULT_HASH,
                reason: Reason::Deploy(mt_stub_hash),
            }
            .to_nep297_event()
            .to_event_log(),
        ]
    );
    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        mt_stub_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_concurrent_upgrades_only_one_succeeds(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.into_account_id()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // Initial deploy so controller has code
    root.gd_approve_and_deploy(&controller_instance, state.code_hash, &DEPLOYER_WASM)
        .await
        .unwrap();
    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        sha256_array(&*DEPLOYER_WASM),
    );

    let old_hash = sha256_array(&*DEPLOYER_WASM);

    // Approve the upgrade before firing concurrent calls
    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(old_hash),
            new_hash: AsHex(sha256_array(&*MT_RECEIVER_STUB_WASM)),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .await
        .unwrap();

    // Fire 10 concurrent upgrade calls all using the same old_hash
    let results = join_all((0..10).map(|_| {
        controller_instance
            .gd_deploy(GDDeployArgs {
                code: MT_RECEIVER_STUB_WASM.to_vec(),
            })
            .deposit(NearToken::from_near(50))
            .gas(Gas::from_tgas(290))
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
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        sha256_array(&*MT_RECEIVER_STUB_WASM),
    );
}

#[rstest]
#[tokio::test]
async fn test_second_approval_overwrites_first(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.into_account_id()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // First approval
    let first_hash = sha256_array(&*DEPLOYER_WASM);
    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(state.code_hash),
            new_hash: AsHex(first_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .await
        .unwrap();

    // Second approval with different new_hash overwrites the first
    let second_hash = sha256_array(&*MT_RECEIVER_STUB_WASM);
    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(state.code_hash),
            new_hash: AsHex(second_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .await
        .unwrap();

    // The second approval should be persisted
    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        second_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_approve_revoke_resets_to_code_hash(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    // State starts with both code_hash and approved_hash set to [0; 32]
    let state = DeployerState::new(root.into_account_id()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // Approve some arbitrary hash
    let arbitrary_hash = sha256_array(&*DEPLOYER_WASM);
    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(state.code_hash),
            new_hash: AsHex(arbitrary_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .await
        .unwrap();
    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        arbitrary_hash,
    );

    // Revoke the approval by resetting approved_hash back to code_hash ([0; 32]).
    // This is a valid use case: the owner changed their mind and wants to cancel
    // a previously approved deployment. Setting approved_hash equal to code_hash
    // effectively disables `gd_deploy` since new code can never hash to [0; 32].
    //
    // NOTE: this also proves that approving a hash already stored as code_hash is
    // allowed — the contract intentionally places no restriction on new_hash.
    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(state.code_hash),
            new_hash: AsHex(state.code_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .await
        .unwrap();
    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        state.code_hash, // back to [0; 32]
    );
}

#[rstest]
#[tokio::test]
async fn test_permissionless_deploy_with_approval(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let (alice, bob) = try_join(
        root.generate_sub_account("alice", NearToken::from_near(100)),
        root.generate_sub_account("bob", NearToken::from_near(100)),
    )
    .await
    .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(alice.into_account_id()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // Owner approves deployment
    let new_code_hash = sha256_array(&*DEPLOYER_WASM);
    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(state.code_hash),
            new_hash: AsHex(new_code_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .sign_with(alice.signer().unwrap())
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        new_code_hash,
    );

    // Non-owner (bob) deploys successfully with matching approved_hash
    controller_instance
        .gd_deploy(GDDeployArgs {
            code: DEPLOYER_WASM.to_vec(),
        })
        .deposit(NearToken::from_near(50))
        .gas(Gas::from_tgas(290))
        .sign_with(bob.signer().unwrap())
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        new_code_hash,
    );

    // approved_hash is reset after deploy
    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        DeployerState::DEFAULT_HASH,
    );

    // Non-owner cannot deploy again without new approval
    controller_instance
        .gd_deploy(GDDeployArgs {
            code: MT_RECEIVER_STUB_WASM.to_vec(),
        })
        .deposit(NearToken::from_near(50))
        .gas(Gas::from_tgas(290))
        .sign_with(bob.signer().unwrap())
        .await
        .assert_err_contains(ERR_NEW_CODE_HASH_MISMATCH);
}

#[rstest]
#[tokio::test]
async fn test_refund_excessive_deposit_attached_to_deploy(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let initial_balance = NearToken::from_near(20);
    let owner = root
        .generate_sub_account("owner", initial_balance)
        .await
        .unwrap();

    assert_eq!(
        owner
            .account(owner.into_account_id())
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .amount,
        initial_balance
    );
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.into_account_id()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        root.account(controller_instance.contract_id())
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .amount,
        NearToken::from_near(0)
    );

    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(storage.code_hash),
            new_hash: AsHex(sha256_array(&*DEPLOYER_WASM)),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap();

    controller_instance
        .gd_deploy(GDDeployArgs {
            code: DEPLOYER_WASM.to_vec(),
        })
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(290))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap();

    let controller_instance_balance = root
        .account(controller_instance.contract_id())
        .finality(Finality::Optimistic)
        .await
        .unwrap()
        .amount;
    assert!(controller_instance_balance < NearToken::from_millinear(900));
}

#[rstest]
#[tokio::test]
async fn test_state_init_pre_approve_allows_immediate_deploy(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let bob = root
        .generate_sub_account("bob", NearToken::from_near(100))
        .await
        .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    // Pre-set approved_hash so gd_deploy can be called immediately without gd_approve
    let state = DeployerState::new(root.into_account_id())
        .with_index(unique_index)
        .pre_approve(sha256_array(&*DEPLOYER_WASM));

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    assert!(
        bob.into_account_id()
            != controller_instance
                .gd_owner_id()
                .finality(Finality::Optimistic)
                .await
                .unwrap()
    );

    controller_instance
        .gd_deploy(GDDeployArgs {
            code: DEPLOYER_WASM.to_vec(),
        })
        .deposit(NearToken::from_near(50))
        .gas(Gas::from_tgas(290))
        .sign_with(bob.signer().unwrap())
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        sha256_array(&*DEPLOYER_WASM),
    );

    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        DeployerState::DEFAULT_HASH,
    );
}

#[rstest]
#[tokio::test]
async fn test_state_init_same_code_hash_and_pre_approve_allows_deploy(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let dummy_wasm: Vec<u8> = vec![1u8; 64];
    let dummy_hash = sha256_array(&dummy_wasm);

    // State where code_hash == approved_hash == hash(dummy_wasm)
    let mut state = DeployerState::new(root.into_account_id())
        .with_index(unique_index)
        .pre_approve(dummy_hash);
    state.code_hash = dummy_hash;

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        dummy_hash
    );
    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        dummy_hash,
    );

    // Deploy should succeed even though code_hash == approved_hash
    controller_instance
        .gd_deploy(GDDeployArgs { code: dummy_wasm })
        .deposit(NearToken::from_near(50))
        .gas(Gas::from_tgas(290))
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        dummy_hash,
    );

    // Approval should be cleared after deploy
    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        DeployerState::DEFAULT_HASH,
    );
}

#[rstest]
#[tokio::test]
async fn test_post_deploy_does_not_run_on_failed_deploy(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let initial_balance = NearToken::from_near(2);
    let owner = root
        .generate_sub_account("dummy2", initial_balance)
        .await
        .unwrap();

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.into_account_id()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    // Instance has 0 balance
    assert_eq!(
        root.account(controller_instance.contract_id())
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .amount,
        NearToken::from_near(0)
    );

    let new_hash = sha256_array(&*DEPLOYER_WASM);

    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(storage.code_hash),
            new_hash: AsHex(new_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        new_hash,
    );

    // Deploy with insufficient deposit → LackBalanceForState
    // The transaction fails, so gd_post_deploy should NOT run
    controller_instance
        .gd_deploy(GDDeployArgs {
            code: DEPLOYER_WASM.to_vec(),
        })
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(290))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap_err();

    // code_hash must NOT have been updated by gd_post_deploy
    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        storage.code_hash,
    );

    // approved_hash must NOT have been cleared by gd_post_deploy
    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        new_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_retry_approve_and_deploy_after_insufficient_deposit(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let owner = root
        .generate_sub_account("retry", NearToken::from_near(100))
        .await
        .unwrap();

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.into_account_id()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    let new_hash = sha256_array(&*DEPLOYER_WASM);

    // First attempt: batch gd_approve + gd_deploy with insufficient deposit (1 NEAR)
    // Both actions are in the same receipt → batch fails atomically, gd_approve reverted too
    owner
        .transaction(controller_instance.contract_id())
        .call("gd_approve")
        .args(GDApproveArgs {
            old_hash: AsHex(storage.code_hash),
            new_hash: AsHex(new_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .call("gd_deploy")
        .args_raw(DEPLOYER_WASM.to_vec())
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(290))
        .send()
        .await
        .assert_err_contains("lacks 14.03 NEAR for state");

    // code_hash unchanged after failed deploy
    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        storage.code_hash,
    );

    // Retry with sufficient deposit
    root.gd_approve_and_deploy(&controller_instance, storage.code_hash, &DEPLOYER_WASM)
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        new_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_post_deploy_fails_when_approval_changed(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.into_account_id()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    // Initial deploy so controller has real code
    root.gd_approve_and_deploy(&controller_instance, state.code_hash, &DEPLOYER_WASM)
        .await
        .unwrap();

    let deployer_hash = sha256_array(&*DEPLOYER_WASM);
    let mt_stub_hash = sha256_array(&*MT_RECEIVER_STUB_WASM);
    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        deployer_hash,
    );

    // Approve re-deploying the same DEPLOYER_WASM code. This way, after the deploy
    // promise succeeds, the controller still runs the deployer code and we can query state.
    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(deployer_hash),
            new_hash: AsHex(deployer_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .await
        .unwrap();

    // Batch transaction:
    //   Action 1: gd_deploy(DEPLOYER_WASM) — passes approved_hash check, creates deploy+callback
    //   Action 2: gd_approve(deployer_hash, mt_stub_hash) — changes approved_hash to mt_stub_hash
    //
    // In NEAR, batch actions execute sequentially in the same receipt, but promise receipts
    // from action 1 execute in later blocks — so action 2's state change is visible to the
    // callback. The callback sees approved_hash == mt_stub_hash != deployer_hash and panics.
    //
    // The callback failure causes the FinalExecutionStatus to be Failure.
    root.transaction(controller_instance.contract_id())
        .call("gd_deploy")
        .args_raw(DEPLOYER_WASM.to_vec())
        .deposit(NearToken::from_near(50))
        .gas(Gas::from_tgas(140))
        .call("gd_approve")
        .args(GDApproveArgs {
            old_hash: AsHex(deployer_hash),
            new_hash: AsHex(mt_stub_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .wait_until(TxExecutionStatus::Final)
        .send()
        .await
        .unwrap_err(); // gd_post_deploy callback panics → FinalExecutionStatus::Failure

    // code_hash should be unchanged — the callback was rejected so state was not updated
    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        deployer_hash,
    );

    // approved_hash should be mt_stub_hash (set by gd_approve in action 2)
    assert_eq!(
        controller_instance
            .gd_approved_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        mt_stub_hash,
    );
}

#[rstest]
#[tokio::test]
async fn test_deploy_with_zero_deposit_and_prefunded_account(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let owner = root
        .generate_sub_account("prefund", NearToken::from_near(10))
        .await
        .unwrap();

    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();
    let storage = DeployerState::new(owner.into_account_id()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        root.account(controller_instance.contract_id())
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .amount,
        NearToken::from_near(0)
    );

    // Pre-fund the deterministic account so it has enough for storage
    root.transaction(controller_instance.contract_id())
        .transfer(NearToken::from_near(50))
        .await
        .unwrap();

    assert_eq!(
        root.account(controller_instance.contract_id())
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .amount,
        NearToken::from_near(50)
    );

    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(storage.code_hash),
            new_hash: AsHex(sha256_array(&*DEPLOYER_WASM)),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap();

    controller_instance
        .gd_deploy(GDDeployArgs {
            code: DEPLOYER_WASM.to_vec(),
        })
        .deposit(NearToken::from_yoctonear(0))
        .gas(Gas::from_tgas(290))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        sha256_array(&*DEPLOYER_WASM),
    );
}

#[rstest]
#[tokio::test]
async fn test_gd_deploy_accepts_raw_bytes(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox;
    let owner = root
        .generate_sub_account("rawbytes", NearToken::from_near(200))
        .await
        .unwrap();
    let storage = DeployerState::new(owner.into_account_id()).with_index(unique_index);

    let controller_instance = root
        .deploy_gd_instance(deployer_env.deployer_global_id.clone(), storage.clone())
        .await
        .unwrap();

    controller_instance
        .gd_approve(GDApproveArgs {
            old_hash: AsHex(storage.code_hash),
            new_hash: AsHex(sha256_array(&*DEPLOYER_WASM)),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap();

    controller_instance
        .gd_deploy(GDDeployArgs {
            code: DEPLOYER_WASM.to_vec(),
        })
        .deposit(NearToken::from_near(50))
        .gas(Gas::from_tgas(290))
        .sign_with(owner.signer().unwrap())
        .await
        .unwrap();

    assert_eq!(
        controller_instance
            .gd_code_hash()
            .finality(Finality::Optimistic)
            .await
            .unwrap()
            .0,
        sha256_array(&*DEPLOYER_WASM),
    );
}
