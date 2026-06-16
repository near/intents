mod nonce;
mod signer;

use anyhow::Result;
use defuse::{
    contract::{Role, config::DefuseConfig},
    simulation_output::SimulationOutput,
};
use defuse_core::{Nonce, PublicKey, Salt, fees::Pips, payload::multi::MultiPayload};
use defuse_serde_utils::base64::AsBase64;
use near_kit::{Action, Final, FunctionCallAction, Near, NearToken};
use near_sdk::{
    AccountId,
    json_types::U128,
    serde::{Deserialize, Serialize},
    serde_json::json,
};
use std::collections::{HashMap, HashSet};

pub use nonce::*;
pub use signer::*;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtBatchBalanceOfArgs {
    pub account_id: AccountId,
    pub token_ids: Vec<String>,
}

use crate::{account::Account, extensions::DEFAULT_GAS, outcome::SuccessfulExecutionOutcome};

pub use defuse::contract;
pub use defuse::core;
pub use defuse::tokens;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct HasPublicKeyArgs {
    pub account_id: AccountId,
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct IsNonceUsedArgs {
    pub account_id: AccountId,
    pub nonce: AsBase64<Nonce>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PublicKeyArgs {
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaltArgs {
    pub salt: Salt,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct InvalidateSaltArgs {
    pub salts: Vec<Salt>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FeeArgs {
    pub fee: Pips,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FeeCollectorArgs {
    pub fee_collector: AccountId,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AclRoleArgs {
    pub role: Role,
    pub account_id: AccountId,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MultiPayloadArgs {
    pub signed: Vec<MultiPayload>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct CleanupNoncesArgs {
    pub nonces: Vec<(AccountId, Vec<AsBase64<Nonce>>)>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ForcePublicKeysArgs {
    pub public_keys: HashMap<AccountId, HashSet<PublicKey>>,
}

// TODO: may be also make ext helpers for view methods?
#[near_kit::contract]
pub trait Defuse {
    fn fee(&self) -> Pips;
    fn fee_collector(&self) -> AccountId;

    fn has_public_key(&self, args: HasPublicKeyArgs) -> bool;
    fn public_keys_of(&self, account_id: AccountId) -> HashSet<PublicKey>;
    fn is_nonce_used(&self, args: IsNonceUsedArgs) -> bool;
    fn is_auth_by_predecessor_id_enabled(&self, account_id: AccountId) -> bool;
    fn is_account_locked(&self, account_id: AccountId) -> bool;
    fn mt_batch_balance_of(&self, args: MtBatchBalanceOfArgs) -> Vec<U128>;

    #[call]
    fn add_public_key(&mut self, args: PublicKeyArgs);

    #[call]
    fn remove_public_key(&mut self, args: PublicKeyArgs);

    #[call]
    fn disable_auth_by_predecessor_id(&mut self);

    #[call]
    fn set_fee(&mut self, args: FeeArgs);

    #[call]
    fn set_fee_collector(&mut self, args: FeeCollectorArgs);

    fn acl_has_role(&self, args: AclRoleArgs) -> bool;

    #[call]
    fn acl_grant_role(&mut self, args: AclRoleArgs) -> Option<bool>;

    fn current_salt(&self) -> Salt;
    fn is_valid_salt(&self, salt: SaltArgs) -> bool;

    #[call]
    fn update_current_salt(&mut self) -> Salt;

    #[call]
    fn invalidate_salts(&mut self, args: InvalidateSaltArgs) -> Salt;

    #[call]
    fn execute_intents(&mut self, args: MultiPayloadArgs);

    fn simulate_intents(&self, args: MultiPayloadArgs) -> SimulationOutput;

    #[call]
    fn add_relayer_key(&mut self, args: PublicKeyArgs);

    #[call]
    fn do_add_relayer_key(&mut self, args: PublicKeyArgs);

    #[call]
    fn delete_relayer_key(&mut self, args: PublicKeyArgs);

    #[call]
    fn cleanup_nonces(&mut self, args: CleanupNoncesArgs);

    #[call]
    fn force_add_public_keys(&mut self, args: ForcePublicKeysArgs);

    #[call]
    fn force_remove_public_keys(&mut self, args: ForcePublicKeysArgs);
}

pub trait DefuseExt {
    async fn defuse_add_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_remove_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_disable_auth_by_predecessor_id(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_set_fee(
        &self,
        defuse: impl Into<AccountId>,
        fee: Pips,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_set_fee_collector(
        &self,
        defuse: impl Into<AccountId>,
        fee_collector: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_acl_grant_role(
        &self,
        defuse: impl Into<AccountId>,
        role: Role,
        account_id: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_update_current_salt(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<(SuccessfulExecutionOutcome, Salt)>;

    async fn defuse_invalidate_salts(
        &self,
        defuse: impl Into<AccountId>,
        salts: impl IntoIterator<Item = Salt>,
    ) -> Result<(SuccessfulExecutionOutcome, Salt)>;

    async fn defuse_execute_intents(
        &self,
        defuse: impl Into<AccountId>,
        signed: impl IntoIterator<Item = MultiPayload>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_simulate_and_execute_intents(
        &self,
        defuse: impl Into<AccountId>,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_add_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_delete_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_cleanup_nonces(
        &self,
        defuse: impl Into<AccountId>,
        nonces: impl IntoIterator<Item = (AccountId, Vec<Nonce>)>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_force_add_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: HashMap<AccountId, HashSet<PublicKey>>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_force_remove_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: HashMap<AccountId, HashSet<PublicKey>>,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl DefuseExt for Near {
    async fn defuse_add_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::add_public_key(PublicKeyArgs { public_key })
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_remove_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::remove_public_key(PublicKeyArgs { public_key })
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_disable_auth_by_predecessor_id(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::disable_auth_by_predecessor_id()
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_set_fee(
        &self,
        defuse: impl Into<AccountId>,
        fee: Pips,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::set_fee(FeeArgs { fee })
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_set_fee_collector(
        &self,
        defuse: impl Into<AccountId>,
        fee_collector: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::set_fee_collector(FeeCollectorArgs {
                    fee_collector: fee_collector.into(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_acl_grant_role(
        &self,
        defuse: impl Into<AccountId>,
        role: Role,
        account_id: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::acl_grant_role(AclRoleArgs {
                    role,
                    account_id: account_id.into(),
                })
                .gas(DEFAULT_GAS),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_update_current_salt(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<(SuccessfulExecutionOutcome, Salt)> {
        let outcome = self
            .transaction(defuse.into())
            .add_action(
                Defuse::update_current_salt()
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;
        let salt = outcome.json::<Salt>()?;
        Ok((outcome.try_into()?, salt))
    }

    async fn defuse_invalidate_salts(
        &self,
        defuse: impl Into<AccountId>,
        salts: impl IntoIterator<Item = Salt>,
    ) -> Result<(SuccessfulExecutionOutcome, Salt)> {
        let outcome = self
            .transaction(defuse.into())
            .add_action(
                Defuse::invalidate_salts(InvalidateSaltArgs {
                    salts: salts.into_iter().collect(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;
        let salt = outcome.json::<Salt>()?;
        Ok((outcome.try_into()?, salt))
    }

    async fn defuse_execute_intents(
        &self,
        defuse: impl Into<AccountId>,
        signed: impl IntoIterator<Item = MultiPayload>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::execute_intents(MultiPayloadArgs {
                    signed: signed.into_iter().collect(),
                })
                .gas(DEFAULT_GAS),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_simulate_and_execute_intents(
        &self,
        defuse: impl Into<AccountId>,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> Result<SuccessfulExecutionOutcome> {
        let defuse_id = defuse.into();
        let signed: Vec<MultiPayload> = intents.into_iter().collect();

        let _simulation_result = self
            .contract::<Defuse>(defuse_id.clone())
            .simulate_intents(MultiPayloadArgs {
                signed: signed.clone(),
            })
            .await?;

        self.defuse_execute_intents(defuse_id, signed).await
    }

    async fn defuse_add_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::add_relayer_key(PublicKeyArgs { public_key })
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_delete_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::delete_relayer_key(PublicKeyArgs { public_key })
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_cleanup_nonces(
        &self,
        defuse: impl Into<AccountId>,
        nonces: impl IntoIterator<Item = (AccountId, Vec<Nonce>)>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::cleanup_nonces(CleanupNoncesArgs {
                    nonces: nonces
                        .into_iter()
                        .map(|(account_id, ns)| {
                            (account_id, ns.into_iter().map(Into::into).collect())
                        })
                        .collect(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_force_add_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: HashMap<AccountId, HashSet<PublicKey>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::force_add_public_keys(ForcePublicKeysArgs { public_keys })
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_force_remove_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: HashMap<AccountId, HashSet<PublicKey>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(
                Defuse::force_remove_public_keys(ForcePublicKeysArgs { public_keys })
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }
}

impl DefuseClient {
    pub async fn query_is_nonce_used(
        &self,
        account_id: impl Into<AccountId>,
        nonce: &Nonce,
    ) -> Result<bool> {
        Ok(self
            .is_nonce_used(IsNonceUsedArgs {
                account_id: account_id.into(),
                nonce: (*nonce).into(),
            })
            .await?)
    }

    pub async fn query_has_public_key(
        &self,
        account_id: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> Result<bool> {
        Ok(self
            .has_public_key(HasPublicKeyArgs {
                account_id: account_id.into(),
                public_key: *public_key,
            })
            .await?)
    }

    pub async fn query_balance_of(
        &self,
        account_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Vec<u128>> {
        Ok(self
            .mt_batch_balance_of(MtBatchBalanceOfArgs {
                account_id: account_id.into(),
                token_ids: token_ids.into_iter().map(Into::into).collect(),
            })
            .await?
            .into_iter()
            .map(|v| v.0)
            .collect())
    }
}

pub trait DefuseDeployerExt {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> DefuseClient;
}

impl DefuseDeployerExt for Near {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> DefuseClient {
        let account = self
            .create_subaccount(name, NearToken::from_near(100))
            .await;

        let action = FunctionCallAction {
            method_name: "new".to_string(),
            args: json!({"config" : config}).to_string().as_bytes().to_vec(),
            gas: DEFAULT_GAS,
            deposit: NearToken::from_near(0),
        };

        account
            .deploy(wasm)
            .add_action(Action::FunctionCall(action))
            .wait_until(Final)
            .await
            .unwrap()
            .result()
            .unwrap();

        self.contract::<Defuse>(account.account_id())
    }
}
