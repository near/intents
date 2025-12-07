mod env;
use std::time::Duration;

use defuse_transfer_auth::storage::{ContractStorage, State};
use env::{AccountExt, BaseEnv};
use near_sdk::{
    NearToken,
    state_init::{StateInit, StateInitV1},
};

const TIMEOUT: Duration = Duration::from_secs(60);
const INIT_BALANCE: NearToken = NearToken::from_near(100);

#[tokio::test]
async fn transfer_auth_global_deployment() -> anyhow::Result<()> {
    let env = BaseEnv::new().await?;
    let root = env.root();

    let (solver1, solver2, escrow, auth_contract, relay, proxy) = futures::try_join!(
        root.create_subaccount("solver1", INIT_BALANCE),
        root.create_subaccount("solver2", INIT_BALANCE),
        root.create_subaccount("escrow", INIT_BALANCE),
        root.create_subaccount("auth-contract", INIT_BALANCE),
        root.create_subaccount("auth-callee", INIT_BALANCE),
        root.create_subaccount("proxy", INIT_BALANCE),
    )?;


    let solver1_raw_state = ContractStorage::init_state(State {
            solver_id: solver1.id(),
            escrow_contract_id: escrow.id(),
            auth_contract: auth_contract.id(),
            auth_callee: relay.id(),
            querier: proxy.id(),
            msg_hash: [0; 32],
        })
        .unwrap();
    let solver1_state_init = StateInit::V1(StateInitV1 {
        code: near_sdk::GlobalContractId::AccountId(env.transfer_auth_global.clone()),
        data: solver1_raw_state.clone()
    });


    let solver2_raw_state = ContractStorage::init_state(State {
            solver_id: solver2.id(),
            escrow_contract_id: escrow.id(),
            auth_contract: auth_contract.id(),
            auth_callee: relay.id(),
            querier: proxy.id(),
            msg_hash: [0; 32],
        })
        .unwrap();
    let solver2_state_init = StateInit::V1(StateInitV1 {
        code: near_sdk::GlobalContractId::AccountId(env.transfer_auth_global.clone()),
        data: solver2_raw_state.clone(),
    });

    let auth_transfer_for_solver1 = solver1_state_init.derive_account_id();
    let auth_transfer_for_solver2 = solver2_state_init.derive_account_id();

    println!("auth_transfer_for_solver1: {}", auth_transfer_for_solver1);
    println!("auth_transfer_for_solver2: {}", auth_transfer_for_solver2);

    root.tx(auth_transfer_for_solver1.clone())
        .state_init(env.transfer_auth_global.clone(),solver1_raw_state)
        .transfer(NearToken::from_yoctonear(1))
        .await
        .unwrap();


    Ok(())
}
