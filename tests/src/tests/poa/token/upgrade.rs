use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::{AccountId, AccountIdRef};
use near_workspaces::{Account, Contract};

use crate::utils::{Sandbox, account::AccountExt};

use super::token_env::POA_TOKEN_WASM;

const UNVERSIONED_POA_CONTRACT_WASM_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/old-artifacts/unversioned-poa/defuse_poa_token.wasm"
));

struct UpgradeFixture {
    sandbox: Sandbox,
    root: Account,
}

impl UpgradeFixture {
    async fn new() -> Self {
        let sandbox = Sandbox::new().await.unwrap();
        let root = sandbox.root_account().clone();

        Self { sandbox, root }
    }

    async fn deploy_unversioned_poa_token(
        &self,
        id: &str,
        owner_id: Option<&AccountIdRef>,
        metadata: Option<FungibleTokenMetadata>,
    ) -> anyhow::Result<Contract> {
        let contract = self
            .root
            .deploy_contract(id, UNVERSIONED_POA_CONTRACT_WASM_BYTES)
            .await?;

        let mut json_args = serde_json::Map::new();
        if let Some(oid) = owner_id {
            json_args.insert("owner_id".to_string(), serde_json::to_value(oid).unwrap());
        }
        if let Some(md) = metadata {
            json_args.insert("metadata".to_string(), serde_json::to_value(md).unwrap());
        }

        contract
            .call("new")
            .args_json(json_args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(contract)
    }

    async fn upgrade_to_new_poa_token(
        &self,
        id_to_deploy_at: &Account,
    ) -> anyhow::Result<Contract> {
        let contract = id_to_deploy_at.deploy(POA_TOKEN_WASM).await?.unwrap();

        contract
            .call("upgrade_to_versioned")
            .args_json(serde_json::Map::new())
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(contract)
    }
}

#[tokio::test]
async fn upgrade_to_versioned() {
    let fixture = UpgradeFixture::new().await;
    let poa_contract_owner = fixture.sandbox.create_account("owner").await;
    let unversioned_poa_contract = fixture
        .deploy_unversioned_poa_token("old-poa-token", Some(poa_contract_owner.id()), None)
        .await
        .unwrap();

    let new_contract = fixture
        .upgrade_to_new_poa_token(unversioned_poa_contract.as_account())
        .await
        .unwrap();

    let wrapped_token: Option<AccountId> = new_contract
        .call("wrapped_token")
        .view()
        .await
        .unwrap()
        .json()
        .unwrap();

    assert!(wrapped_token.is_none());
}
