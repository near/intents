use std::sync::atomic::{AtomicU32, Ordering};

use defuse_global_deployer::{OwnerProxyState, State as DeployerState};
use defuse_sandbox::{
    api::types::transaction::actions::GlobalContractDeployMode, extensions::{deployer_hash_proxy::DeployerHashProxyExt, global_deployer::{DeployerExt, DeployerViewExt}}, sandbox, Sandbox
};
use defuse_test_utils::wasms::DEPLOYER_HASH_PROXY_WASM;
use near_sdk::{GlobalContractId, NearToken, env::sha256_array};
use rstest::{fixture, rstest};

use crate::utils::wasms::DEPLOYER_WASM;

static SUB_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct HashProxyEnv {
    pub sandbox: Sandbox,
    #[allow(dead_code)]
    pub deployer_global_id: GlobalContractId,
    pub hash_proxy_global_id: GlobalContractId,
}

#[fixture]
pub async fn hash_proxy_env() -> HashProxyEnv {
    let sandbox = sandbox(NearToken::from_near(100_000)).await;
    let root = sandbox.root();

    root.deploy_global_contract(DEPLOYER_WASM.clone(), GlobalContractDeployMode::CodeHash)
        .await
        .unwrap();
    root.deploy_global_contract(
        DEPLOYER_HASH_PROXY_WASM.clone(),
        GlobalContractDeployMode::CodeHash,
    )
    .await
    .unwrap();

    HashProxyEnv {
        sandbox,
        deployer_global_id: GlobalContractId::CodeHash(sha256_array(&*DEPLOYER_WASM).into()),
        hash_proxy_global_id: GlobalContractId::CodeHash(
            sha256_array(&*DEPLOYER_HASH_PROXY_WASM).into(),
        ),
    }
}

#[fixture]
pub fn unique_index() -> u32 {
    SUB_COUNTER.fetch_add(1, Ordering::Relaxed)
}

const DUMMY_WASM: &[u8] = &[1u8; 1024];

#[rstest]
#[tokio::test]
async fn test_hp_approve(
    #[future(awt)] hash_proxy_env: HashProxyEnv,
    unique_index: u32,
) {
    let HashProxyEnv { sandbox, deployer_global_id, hash_proxy_global_id } 
     = hash_proxy_env;

    let root = sandbox.root();
    let alice = root
        .generate_subaccount("alice", NearToken::from_near(1))
        .await
        .unwrap();

    let bob = root
        .generate_subaccount("bob", NearToken::from_near(100))
        .await
        .unwrap();


    let deployer_state = DeployerState::new_with_contract(alice.id().clone(), hash_proxy_global_id.clone() ,unique_index);
    let deployer_instance = root.deploy_instance(
        deployer_global_id.clone(),
        deployer_state.clone()
    )
    .await.unwrap();


    let proxy_state = OwnerProxyState{ 
        owner_id: alice.id().clone(), 
        old_hash: deployer_state.code_hash, 
        new_hash: sha256_array(DUMMY_WASM),
        deployer_instance: deployer_instance.id().clone()
    };

    assert_eq!(deployer_instance.gd_code_hash().await.unwrap(), DeployerState::DEFAULT_HASH);

    let hash_proxy_instance = alice.deploy_and_approve(hash_proxy_global_id.clone(),proxy_state.into())
    .await
    .unwrap();

    bob.hp_exec(hash_proxy_instance.id(), DUMMY_WASM).await.unwrap();

    assert_eq!(deployer_instance.gd_code_hash().await.unwrap(), sha256_array(DUMMY_WASM));



    // let dummy_hash = sha256_array(&*DEPLOYER_WASM);
    //
    // let proxy_state = OwnerProxyState::new(
    //     root.id().clone(),
    //     DeployerState::DEFAULT_HASH,
    //     dummy_hash,
    //     deployer_instance_id,
    // );
    // let hash_proxy_state = HashProxyState::new(proxy_state);
    //
    // let hash_proxy_instance = root
    //     .deploy_hash_proxy_instance(hash_proxy_global_id, hash_proxy_state)
    //     .await
    //     .unwrap();
    //
    // let result = root
    //     .hp_approve(hash_proxy_instance.id())
    //     .await
    //     .unwrap();
    //
    // let expected_event = Event::Approved {
    //     old_hash: DeployerState::DEFAULT_HASH,
    //     new_hash: dummy_hash,
    // };
    // assert!(
    //     result
    //         .logs()
    //         .contains(&expected_event.to_nep297_event().to_event_log().as_str()),
    //     "Expected Approved event in logs: {:?}",
    //     result.logs(),
    // );
}
