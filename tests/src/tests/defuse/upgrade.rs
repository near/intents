use defuse::core::crypto::PublicKey;
use near_sdk::AccountId;
use randomness::{Rng, make_true_rng};

use crate::{tests::defuse::accounts::AccountManagerExt, utils::mt::MtExt};

use super::DEFUSE_WASM;

#[ignore = "only for simple upgrades"]
#[tokio::test]
async fn upgrade() {
    let old_contract_id: AccountId = "intents.near".parse().unwrap();
    let mainnet = near_workspaces::mainnet()
        .rpc_addr("https://nearrpc.aurora.dev")
        .await
        .unwrap();

    let sandbox = near_workspaces::sandbox().await.unwrap();
    let new_contract = sandbox
        .import_contract(&old_contract_id, &mainnet)
        .with_data()
        .transact()
        .await
        .unwrap();

    new_contract
        .as_account()
        .deploy(&DEFUSE_WASM)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    assert_eq!(
        new_contract
            .mt_balance_of(
                &"user.near".parse().unwrap(),
                &"non-existent-token".to_string(),
            )
            .await
            .unwrap(),
        0
    );

    for public_key in [
        PublicKey::Ed25519(make_true_rng().random()),
        PublicKey::Secp256k1(make_true_rng().random()),
        PublicKey::P256(make_true_rng().random()),
    ] {
        assert!(
            new_contract
                .has_public_key(&public_key.to_implicit_account_id(), &public_key)
                .await
                .unwrap()
        );

        assert!(
            !new_contract
                .has_public_key(new_contract.id(), &public_key)
                .await
                .unwrap()
        );
    }
}
