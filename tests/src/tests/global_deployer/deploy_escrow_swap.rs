use super::*;

use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use defuse_escrow_swap::{ContractStorage, Deadline, OverrideSend, Params};
use defuse_sandbox::{
    SigningAccount,
    extensions::{escrow::EscrowExtView, mt_receiver::MtReceiverStubExtView},
};
use defuse_test_utils::wasms::ESCROW_SWAP_WASM;
use near_sdk::state_init::{StateInit, StateInitV1};

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

    let state = DeployerState::new(root.id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    root.gd_approve_and_deploy(controller_instance.id(), state.code_hash, &DEPLOYER_WASM)
        .await
        .unwrap();
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let upgradable_state = DeployerState::new(alice.id().clone());
    let upgradable_controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), upgradable_state.clone())
        .await
        .unwrap();

    alice
        .gd_approve_and_deploy(
            upgradable_controller_instance.id(),
            upgradable_state.code_hash,
            &DEPLOYER_WASM,
        )
        .await
        .unwrap();
    assert_eq!(
        upgradable_controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let escrow_state = DeployerState::new(bob.id().clone());
    let escrow_controller_instance = root
        .deploy_instance(
            GlobalContractId::AccountId(upgradable_controller_instance.id().clone()),
            escrow_state.clone(),
        )
        .await
        .unwrap();

    bob.gd_approve_and_deploy(
        escrow_controller_instance.id(),
        escrow_state.code_hash,
        &ESCROW_SWAP_WASM,
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

    let state = DeployerState::new(root.id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    root.gd_approve_and_deploy(controller_instance.id(), state.code_hash, &DEPLOYER_WASM)
        .await
        .unwrap();
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let upgradable_state = DeployerState::new(alice.id().clone());
    let upgradable_controller_instance = root
        .deploy_instance(deployer_code_hash_id.clone(), upgradable_state.clone())
        .await
        .unwrap();

    alice
        .gd_approve_and_deploy(
            upgradable_controller_instance.id(),
            upgradable_state.code_hash,
            &DEPLOYER_WASM,
        )
        .await
        .unwrap();
    assert_eq!(
        upgradable_controller_instance.gd_code_hash().await.unwrap(),
        sha256_array(&*DEPLOYER_WASM),
    );

    let escrow_state = DeployerState::new(bob.id().clone());
    let escrow_controller_instance = root
        .deploy_instance(
            GlobalContractId::AccountId(upgradable_controller_instance.id().clone()),
            escrow_state.clone(),
        )
        .await
        .unwrap();

    bob.gd_approve_and_deploy(
        escrow_controller_instance.id(),
        escrow_state.code_hash,
        &MT_RECEIVER_STUB_WASM,
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

    bob.gd_approve(
        escrow_controller_instance.id(),
        sha256_array(&*MT_RECEIVER_STUB_WASM),
        sha256_array(&*ESCROW_SWAP_WASM),
    )
    .await
    .unwrap();

    bob.gd_deploy(
        escrow_controller_instance.id(),
        sha256_array(&*MT_RECEIVER_STUB_WASM),
        &ESCROW_SWAP_WASM,
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
