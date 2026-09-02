#![allow(dead_code)]

use near_sdk::NearToken;
use near_workspaces::{Account, Contract};
use sha2::{Digest, Sha256};

pub trait AccountExt {
    async fn deploy_contract(&self, account_id: &str, wasm: &[u8]) -> anyhow::Result<Contract>;
}

/// Hex-encoded sha256 of a wasm blob.
fn wasm_code_hash(wasm: &[u8]) -> String {
    hex::encode(Sha256::digest(wasm))
}

impl AccountExt for Account {
    async fn deploy_contract(&self, account_id: &str, wasm: &[u8]) -> anyhow::Result<Contract> {
        let contract = self
            .create_subaccount(account_id)
            .initial_balance(NearToken::from_near(15))
            .transact()
            .await?
            .into_result()?
            .deploy(wasm.as_ref())
            .await?
            .into_result()?;

        println!(
            "[deploy] account={} wasm_hash={} ({} bytes)",
            contract.id(),
            wasm_code_hash(wasm),
            wasm.len(),
        );

        Ok(contract)
    }
}
