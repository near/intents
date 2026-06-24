use super::*;

use defuse_sandbox::{
    extensions::{
        escrow::{
            Escrow,
            contract::{ContractStorage, Deadline, OverrideSend, Params},
        },
        mt_receiver::MtReceiverStub,
    },
    helpers::sha256_hash,
    kit::AccountId,
    nep616::DeployDeterministicAccountExt,
};
use defuse_test_utils::wasms::ESCROW_SWAP_WASM;

use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

fn dummy_escrow_params(root: &AccountId) -> Params {
    let maker = root.sub_account("maker").unwrap();
    let src_token = root.sub_account("src_token").unwrap();
    let dst_token = root.sub_account("dst_token").unwrap();

    Params {
        maker,
        src_token: format!("nep141:{src_token}").parse().unwrap(),
        dst_token: format!("nep141:{dst_token}").parse().unwrap(),
        price: "1".parse().unwrap(),
        deadline: Deadline::timeout(Duration::from_hours(1)),
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
    let root = deployer_env.root;
    let alice = root
        .create_subaccount("alice", NearToken::from_near(100))
        .await;
    let bob = root
        .create_subaccount("bob", NearToken::from_near(100))
        .await;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.account_id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    root.gd_approve_and_deploy(
        controller_instance.contract_id(),
        state.code_hash,
        &*DEPLOYER_WASM,
    )
    .await
    .unwrap();

    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        sha256_hash(&*DEPLOYER_WASM),
    );

    let upgradable_state = DeployerState::new(alice.account_id().clone());
    let upgradable_controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), upgradable_state.clone())
        .await
        .unwrap();

    alice
        .gd_approve_and_deploy(
            upgradable_controller_instance.contract_id(),
            upgradable_state.code_hash,
            &*DEPLOYER_WASM,
        )
        .await
        .unwrap();
    assert_eq!(
        upgradable_controller_instance
            .gd_code_hash()
            .await
            .unwrap()
            .0,
        sha256_hash(&*DEPLOYER_WASM),
    );

    let escrow_state = DeployerState::new(bob.account_id().clone());
    let escrow_controller_instance = root
        .deploy_gd_instance(
            GlobalContractId::AccountId(upgradable_controller_instance.contract_id().clone()),
            escrow_state.clone(),
        )
        .await
        .unwrap();

    bob.gd_approve_and_deploy(
        escrow_controller_instance.contract_id(),
        escrow_state.code_hash,
        &*ESCROW_SWAP_WASM,
    )
    .await
    .unwrap();
    assert_eq!(
        escrow_controller_instance.gd_code_hash().await.unwrap().0,
        sha256_hash(&*ESCROW_SWAP_WASM),
    );

    let escrow_instance_params = dummy_escrow_params(root.account_id());
    let escrow_instance = {
        let account_id = root
            .deploy_deterministic_account(
                GlobalContractId::AccountId(escrow_controller_instance.contract_id().clone()),
                ContractStorage::init_state(&escrow_instance_params).unwrap(),
                NearToken::ZERO,
            )
            .await
            .unwrap();
        root.contract::<Escrow>(account_id)
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
    let root = deployer_env.root;
    let alice = root
        .create_subaccount("alice", NearToken::from_near(100))
        .await;
    let bob = root
        .create_subaccount("bob", NearToken::from_near(500))
        .await;
    let deployer_code_hash_id = deployer_env.deployer_global_id.clone();

    let state = DeployerState::new(root.account_id().clone()).with_index(unique_index);
    let controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), state.clone())
        .await
        .unwrap();

    root.gd_approve_and_deploy(
        controller_instance.contract_id(),
        state.code_hash,
        &*DEPLOYER_WASM,
    )
    .await
    .unwrap();
    assert_eq!(
        controller_instance.gd_code_hash().await.unwrap().0,
        sha256_hash(&*DEPLOYER_WASM),
    );

    let upgradable_state = DeployerState::new(alice.account_id().clone());
    let upgradable_controller_instance = root
        .deploy_gd_instance(deployer_code_hash_id.clone(), upgradable_state.clone())
        .await
        .unwrap();

    alice
        .gd_approve_and_deploy(
            upgradable_controller_instance.contract_id(),
            upgradable_state.code_hash,
            &*DEPLOYER_WASM,
        )
        .await
        .unwrap();
    assert_eq!(
        upgradable_controller_instance
            .gd_code_hash()
            .await
            .unwrap()
            .0,
        sha256_hash(&*DEPLOYER_WASM),
    );

    let escrow_state = DeployerState::new(bob.account_id().clone());
    let escrow_controller_instance = root
        .deploy_gd_instance(
            GlobalContractId::AccountId(upgradable_controller_instance.contract_id().clone()),
            escrow_state.clone(),
        )
        .await
        .unwrap();

    bob.gd_approve_and_deploy(
        escrow_controller_instance.contract_id(),
        escrow_state.code_hash,
        &*MT_RECEIVER_STUB_WASM,
    )
    .await
    .unwrap();
    assert_eq!(
        escrow_controller_instance.gd_code_hash().await.unwrap().0,
        sha256_hash(&*MT_RECEIVER_STUB_WASM),
    );

    let escrow_instance_params = dummy_escrow_params(root.account_id());
    let escrow_instance = {
        let account_id = root
            .deploy_deterministic_account(
                GlobalContractId::AccountId(escrow_controller_instance.contract_id().clone()),
                ContractStorage::init_state(&escrow_instance_params).unwrap(),
                NearToken::ZERO,
            )
            .await
            .unwrap();
        root.contract::<Escrow>(account_id)
    };

    escrow_instance
        .es_view()
        .await
        .expect_err("escrow should not have `es_view` method");

    // near-sdk returns empty bytes for void methods; near-kit fails to JSON-parse them.
    // Only an Rpc error means the method doesn't exist.
    root.contract::<MtReceiverStub>(escrow_instance.contract_id().clone())
        .dummy_method()
        .await
        .unwrap();

    bob.gd_approve(
        escrow_controller_instance.contract_id(),
        sha256_hash(&*MT_RECEIVER_STUB_WASM),
        sha256_hash(&*ESCROW_SWAP_WASM),
    )
    .await
    .unwrap();

    bob.gd_deploy(
        escrow_controller_instance.contract_id(),
        &*ESCROW_SWAP_WASM,
        NearToken::from_near(50),
    )
    .await
    .unwrap();
    assert_eq!(
        escrow_controller_instance.gd_code_hash().await.unwrap().0,
        sha256_hash(&*ESCROW_SWAP_WASM),
    );
    let storage = escrow_instance
        .es_view()
        .await
        .expect("escrow should have `es_view` method");
    storage.verify(&escrow_instance_params).unwrap();
    root.contract::<MtReceiverStub>(escrow_instance.contract_id().clone())
        .dummy_method()
        .await
        .expect_err("escrow should not have `dummy_method` method");
}
