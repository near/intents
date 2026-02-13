use near_sdk::AsNep297Event;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::utils::wasms::DEPLOYER_WASM;
use defuse_escrow_swap::{ContractStorage, Deadline, OverrideSend, Params};
use defuse_global_deployer::{
    Event, State as DeployerState,
    error::{ERR_UNAUTHORIZED, ERR_WRONG_CODE_HASH},
};
use defuse_sandbox::extensions::escrow::EscrowExtView;
use defuse_sandbox::extensions::global_deployer::{DeployerExt, DeployerViewExt};
use defuse_sandbox::extensions::mt_receiver::MtReceiverStubExtView;
use defuse_sandbox::{
    Sandbox, SigningAccount, api::types::transaction::actions::GlobalContractDeployMode, sandbox,
    tx::FnCallBuilder,
};
use defuse_test_utils::asserts::ResultAssertsExt;
use defuse_test_utils::wasms::{ESCROW_SWAP_WASM, MT_RECEIVER_STUB_WASM};
use futures::future::join_all;
use near_sdk::{
    GlobalContractId, NearToken,
    env::sha256_array,
    state_init::{StateInit, StateInitV1},
};

use rstest::{fixture, rstest};

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

fn dummy_escrow_params(root: &SigningAccount) -> Params {
    let maker = root.sub_account("maker").unwrap();
    let src_token = root.sub_account("src_token").unwrap();
    let dst_token = root.sub_account("dst_token").unwrap();

    Params {
        maker: maker.id().clone(),
        src_token: format!("nep141:{}", src_token.id()).parse().unwrap(),
        dst_token: format!("nep141:{}", dst_token.id()).parse().unwrap(),
        price: "1".parse().unwrap(),
        deadline: Deadline::timeout(Duration::from_secs(3600)),
        partial_fills_allowed: false,
        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        taker_whitelist: BTreeSet::new(),
        protocol_fees: None,
        integrator_fees: BTreeMap::new(),
        auth_caller: None,
        salt: [0u8; 32],
    }
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

    root.gd_deploy(
        controller_instance.id(),
        &DEPLOYER_WASM,
        DeployerState::DEFAULT_HASH,
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
async fn test_deploy_escrow_swap(#[future(awt)] deployer_env: DeployerEnv, unique_index: u32) {
    let root = deployer_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();
    let bob = root
        .generate_subaccount("bob", NearToken::from_near(100))
        .await
        .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let controller_instance = root
        .deploy_instance(
            deployer_code_hash_id.clone(),
            DeployerState::new(root.id().clone(), unique_index),
        )
        .await
        .unwrap();

    root.gd_deploy(
        controller_instance.id(),
        &DEPLOYER_WASM,
        DeployerState::DEFAULT_HASH,
    )
    .await
    .unwrap();
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let upgradable_controller_instance = root
        .deploy_instance(
            deployer_code_hash_id.clone(),
            DeployerState::new(alice.id().clone(), 0u32),
        )
        .await
        .unwrap();
    alice
        .gd_deploy(
            upgradable_controller_instance.id(),
            &DEPLOYER_WASM,
            DeployerState::DEFAULT_HASH,
        )
        .await
        .unwrap();
    assert_eq!(
        upgradable_controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let escrow_controller_instance = root
        .deploy_instance(
            GlobalContractId::AccountId(upgradable_controller_instance.id().clone()),
            DeployerState::new(bob.id().clone(), 0u32),
        )
        .await
        .unwrap();
    bob.gd_deploy(
        escrow_controller_instance.id(),
        &ESCROW_SWAP_WASM,
        DeployerState::DEFAULT_HASH,
    )
    .await
    .unwrap();
    assert_eq!(
        escrow_controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*ESCROW_SWAP_WASM),
    );

    let escrow_instance_params = dummy_escrow_params(root);
    let escrow_instance = {
        let escrow_account_id = root
            .state_init(
                StateInit::V1(StateInitV1 {
                    code: GlobalContractId::AccountId(escrow_controller_instance.id().clone()),
                    data: ContractStorage::init_state(&escrow_instance_params).unwrap(),
                }),
                NearToken::ZERO,
            )
            .await
            .unwrap();
        defuse_sandbox::Account::new(escrow_account_id, root.network_config().clone())
    };

    // call escrow instance method
    let storage = escrow_instance
        .es_view()
        .await
        .expect("escrow should have `es_view` method");
    storage.verify(&escrow_instance_params).unwrap();
}

#[rstest]
#[tokio::test]
async fn test_deploy_escrow_instance_on_dummy_wasm_then_upgrade_code_to_escrow_using_controller(
    #[future(awt)] deployer_env: DeployerEnv,
    unique_index: u32,
) {
    let root = deployer_env.sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(100))
        .await
        .unwrap();
    let bob = root
        .generate_subaccount("bob", NearToken::from_near(500))
        .await
        .unwrap();
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let controller_instance = root
        .deploy_instance(
            deployer_code_hash_id.clone(),
            DeployerState::new(root.id().clone(), unique_index),
        )
        .await
        .unwrap();

    root.gd_deploy(
        controller_instance.id(),
        &DEPLOYER_WASM,
        DeployerState::DEFAULT_HASH,
    )
    .await
    .unwrap();
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let upgradable_controller_instance = root
        .deploy_instance(
            deployer_code_hash_id.clone(),
            DeployerState::new(alice.id().clone(), 0u32),
        )
        .await
        .unwrap();
    alice
        .gd_deploy(
            upgradable_controller_instance.id(),
            &DEPLOYER_WASM,
            DeployerState::DEFAULT_HASH,
        )
        .await
        .unwrap();
    assert_eq!(
        upgradable_controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let escrow_controller_instance = root
        .deploy_instance(
            GlobalContractId::AccountId(upgradable_controller_instance.id().clone()),
            DeployerState::new(bob.id().clone(), 0u32),
        )
        .await
        .unwrap();

    bob.gd_deploy(
        escrow_controller_instance.id(),
        &MT_RECEIVER_STUB_WASM,
        DeployerState::DEFAULT_HASH,
    )
    .await
    .unwrap();
    assert_eq!(
        escrow_controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*MT_RECEIVER_STUB_WASM),
    );

    let escrow_instance_params = dummy_escrow_params(root);
    let escrow_instance = {
        let escrow_account_id = root
            .state_init(
                StateInit::V1(StateInitV1 {
                    code: GlobalContractId::AccountId(escrow_controller_instance.id().clone()),
                    data: ContractStorage::init_state(&escrow_instance_params).unwrap(),
                }),
                NearToken::ZERO,
            )
            .await
            .unwrap();
        defuse_sandbox::Account::new(escrow_account_id, root.network_config().clone())
    };

    escrow_instance
        .es_view()
        .await
        .expect_err("escrow should not have `es_view` method");
    escrow_instance
        .dummy_method()
        .await
        .expect("escrow should have `dummy_method` method");

    bob.gd_deploy(
        escrow_controller_instance.id(),
        &ESCROW_SWAP_WASM,
        sha256_array(&*MT_RECEIVER_STUB_WASM),
    )
    .await
    .unwrap();
    assert_eq!(
        escrow_controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*ESCROW_SWAP_WASM),
    );
    let storage = escrow_instance
        .es_view()
        .await
        .expect("escrow should have `es_view` method");
    storage.verify(&escrow_instance_params).unwrap();
    escrow_instance
        .dummy_method()
        .await
        .expect_err("escrow should not have `dummy_method` method");
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

    let storage_deposit = NearToken::from_near(1);
    owner
        .tx(controller_instance.id())
        .function_call(
            FnCallBuilder::new("gd_deploy")
                .borsh_args(&(&*DEPLOYER_WASM, DeployerState::DEFAULT_HASH))
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
    bob.gd_deploy(
        controller_instance.id(),
        &DEPLOYER_WASM,
        DeployerState::DEFAULT_HASH,
    )
    .await
    .assert_err_contains(ERR_UNAUTHORIZED);

    bob.gd_transfer_ownership(controller_instance.id(), alice.id())
        .await
        .assert_err_contains(ERR_UNAUTHORIZED);

    let result = alice
        .gd_transfer_ownership(controller_instance.id(), bob.id())
        .await
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            Event::Transfer {
                old_owner_id: alice.id().clone(),
                new_owner_id: bob.id().clone(),
            }
            .to_nep297_event()
            .to_event_log()
        ]
    );

    assert_eq!(
        controller_instance.gd_owner_id().await.unwrap(),
        bob.id().clone()
    );
    alice
        .gd_deploy(
            controller_instance.id(),
            &DEPLOYER_WASM,
            DeployerState::DEFAULT_HASH,
        )
        .await
        .assert_err_contains(ERR_UNAUTHORIZED);
    alice
        .gd_transfer_ownership(controller_instance.id(), bob.id())
        .await
        .assert_err_contains(ERR_UNAUTHORIZED);

    bob.gd_deploy(
        controller_instance.id(),
        &DEPLOYER_WASM,
        DeployerState::DEFAULT_HASH,
    )
    .await
    .unwrap();
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );
    bob.gd_transfer_ownership(controller_instance.id(), alice.id())
        .await
        .unwrap();
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

    let result = root
        .tx(controller_instance.id())
        .function_call(
            FnCallBuilder::new("gd_deploy")
                .borsh_args(&(&*DEPLOYER_WASM, DeployerState::DEFAULT_HASH))
                .with_deposit(NearToken::from_near(50)),
        )
        .await
        .unwrap();

    let expected_event = defuse_global_deployer::Event::Deploy(sha256_array(&*DEPLOYER_WASM));
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
    root.gd_deploy(
        controller_instance.id(),
        &DEPLOYER_WASM,
        DeployerState::DEFAULT_HASH,
    )
    .await
    .unwrap();
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let old_hash = sha256_array(&*DEPLOYER_WASM);

    // Fire 10 concurrent upgrade calls all using the same old_hash
    let results = join_all(
        (0..10).map(|_| root.gd_deploy(controller_instance.id(), &ESCROW_SWAP_WASM, old_hash)),
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
        sha256_array(&*ESCROW_SWAP_WASM),
    );
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
        .tx(controller_instance.id())
        .function_call(
            FnCallBuilder::new("gd_deploy")
                .borsh_args(&(&*DEPLOYER_WASM, DeployerState::DEFAULT_HASH))
                .with_deposit(NearToken::from_near(100)),
        )
        .await
        .unwrap();

    let controller_instance_balance = controller_instance.view().await.unwrap().amount;
    assert!(controller_instance_balance < NearToken::from_millinear(900));
}
