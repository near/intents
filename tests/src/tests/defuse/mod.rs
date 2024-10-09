pub mod accounts;
mod env;
mod intents;
mod tokens;

use std::{collections::HashMap, sync::LazyLock};

use accounts::AccountManagerExt;
use defuse_contract::Role;
use defuse_contracts::{
    defuse::{
        message::{DefuseMessage, SignedDefuseMessage},
        payload::MultiStandardPayload,
    },
    nep413::{Nep413Payload, Nonce},
    utils::{fees::Pips, Deadline},
};
use near_sdk::{borsh::BorshSerialize, AccountId};
use near_workspaces::Contract;
use serde_json::json;

use crate::utils::{account::AccountExt, crypto::Signer, read_wasm};

static DEFUSE_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse_contract"));

pub trait DefuseExt: AccountManagerExt {
    async fn deploy_defuse(
        &self,
        id: &str,
        fee: Pips,
        fee_collector: &AccountId,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
    ) -> anyhow::Result<Contract>;
}

impl DefuseExt for near_workspaces::Account {
    async fn deploy_defuse(
        &self,
        id: &str,
        fee: Pips,
        fee_collector: &AccountId,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
    ) -> anyhow::Result<Contract> {
        let contract = self.deploy_contract(id, &DEFUSE_WASM).await?;
        contract
            .call("new")
            .args_json(json!({
                "fee": fee,
                "fee_collector": fee_collector,
                "super_admins": super_admins.into_iter().collect::<Vec<_>>(),
                "admins": admins
                    .into_iter()
                    .map(|(role, admins)| (role, admins.into_iter().collect::<Vec<_>>()))
                    .collect::<HashMap<_, _>>(),
                "grantees": grantees
                    .into_iter()
                    .map(|(role, grantees)| (role, grantees.into_iter().collect::<Vec<_>>()))
                    .collect::<HashMap<_, _>>(),
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(contract)
    }
}

impl DefuseExt for Contract {
    async fn deploy_defuse(
        &self,
        id: &str,
        fee: Pips,
        fee_collector: &AccountId,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
    ) -> anyhow::Result<Contract> {
        self.as_account()
            .deploy_defuse(id, fee, fee_collector, super_admins, admins, grantees)
            .await
    }
}

pub trait DefuseSigner: Signer {
    fn sign_defuse_message<T>(
        &self,
        defuse_contract: &AccountId,
        message: T,
        nonce: Nonce,
        deadline: Deadline,
    ) -> SignedDefuseMessage<T>
    where
        T: BorshSerialize;
}

impl DefuseSigner for near_workspaces::Account {
    fn sign_defuse_message<T>(
        &self,
        defuse_contract: &AccountId,
        message: T,
        nonce: Nonce,
        deadline: Deadline,
    ) -> SignedDefuseMessage<T>
    where
        T: BorshSerialize,
    {
        self.sign_payload(MultiStandardPayload::Nep413(
            Nep413Payload::new(DefuseMessage {
                signer_id: self.id().clone(),
                deadline,
                message,
            })
            .with_recipient(defuse_contract.to_string())
            .with_nonce(nonce),
        ))
    }
}
