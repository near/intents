mod event;
mod nonce;
mod signer;

use anyhow::Result;
use defuse::{
    contract::{Role, config::DefuseConfig},
    simulation_output::SimulationOutput,
};
use defuse_core::{Nonce, PublicKey, Salt, fees::Pips, intents::auth::AuthCall, payload::multi::MultiPayload};
use defuse_serde_utils::base64::AsBase64;
use near_account_id::AccountId;
use near_kit::{Action, Final, FunctionCallAction, Near, NearToken};
use near_sdk::{AccountIdRef, json_types::U128};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::{base64::Base64, serde_as};
use std::collections::{HashMap, HashSet};

use crate::{
    account::Account,
    extensions::{DEFAULT_GAS, FnCallTransaction},
    outcome::SuccessfulExecutionOutcome,
};

pub use event::*;
pub use nonce::*;
pub use signer::*;

pub use defuse::contract;
pub use defuse::core;
pub use defuse::tokens;

#[derive(Serialize)]
pub struct HasPublicKeyArgs<'a> {
    pub account_id: &'a AccountIdRef,
    pub public_key: &'a PublicKey,
}

#[serde_as]
#[derive(Serialize)]
pub struct IsNonceUsedArgs<'a> {
    pub account_id: &'a AccountIdRef,
    #[serde_as(as = "Base64")]
    pub nonce: &'a Nonce,
}

#[derive(Serialize)]
pub struct PublicKeyArgs {
    pub public_key: PublicKey,
}

#[derive(Serialize)]
pub struct SaltArgs {
    pub salt: Salt,
}

#[derive(Serialize)]
pub struct InvalidateSaltArgs {
    pub salts: Vec<Salt>,
}

#[derive(Serialize)]
pub struct FeeArgs {
    pub fee: Pips,
}

#[derive(Serialize)]
pub struct FeeCollectorArgs<'a> {
    pub fee_collector: &'a AccountIdRef,
}

#[derive(Serialize)]
pub struct AclRoleArgs<'a> {
    pub role: Role,
    pub account_id: &'a AccountIdRef,
}

#[derive(Serialize)]
pub struct MultiPayloadArgs {
    pub signed: Vec<MultiPayload>,
}

#[derive(Serialize)]
pub struct CleanupNoncesArgs {
    pub nonces: Vec<(AccountId, Vec<AsBase64<Nonce>>)>,
}

#[derive(Serialize, Deserialize)]
pub struct ForcePublicKeysArgs {
    pub public_keys: HashMap<AccountId, HashSet<PublicKey>>,
}

#[derive(Serialize)]
pub struct AccountArgs<'a> {
    pub account_id: &'a AccountIdRef,
}

#[derive(Serialize)]
pub struct MultipleAccountsArgs {
    pub account_ids: Vec<AccountId>,
}

#[derive(Serialize)]
pub struct FtWithdrawArgs {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub amount: U128,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize)]
pub struct MtWithdrawArgs {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub token_ids: Vec<defuse_nep245::TokenId>,
    pub amounts: Vec<U128>,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize)]
pub struct NftWithdrawArgs {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub token_id: String,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize)]
pub struct DoAuthCallArgs {
    pub signer_id: AccountId,
    pub auth_call: AuthCall,
}

#[near_kit::contract]
pub trait Defuse {
    fn fee(&self) -> Pips;
    fn fee_collector(&self) -> AccountId;

    fn has_public_key(&self, args: HasPublicKeyArgs) -> bool;
    fn public_keys_of(&self, args: AccountArgs) -> HashSet<PublicKey>;

    fn is_nonce_used(&self, args: IsNonceUsedArgs) -> bool;
    fn is_auth_by_predecessor_id_enabled(&self, args: AccountArgs) -> bool;

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

    fn is_account_locked(&self, args: AccountArgs) -> bool;

    #[call]
    fn force_lock_account(&mut self, args: AccountArgs) -> bool;
    #[call]
    fn force_unlock_account(&mut self, args: AccountArgs) -> bool;
    #[call]
    fn force_disable_auth_by_predecessor_ids(&mut self, args: MultipleAccountsArgs);
    #[call]
    fn force_enable_auth_by_predecessor_ids(&mut self, args: MultipleAccountsArgs);

    #[call]
    fn ft_withdraw(&mut self, args: FtWithdrawArgs) -> U128;
    #[call]
    fn mt_withdraw(&mut self, args: MtWithdrawArgs) -> Vec<U128>;
    #[call]
    fn nft_withdraw(&mut self, args: NftWithdrawArgs) -> bool;

    #[call]
    fn do_auth_call(&mut self, args: DoAuthCallArgs);
}

pub trait DefuseExt {
    async fn defuse_add_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_remove_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
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
        public_key: impl Into<PublicKey>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_delete_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
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

    async fn defuse_force_lock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl Into<AccountId>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)>;

    async fn defuse_force_unlock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl Into<AccountId>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)>;

    async fn defuse_force_disable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_force_enable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_ft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, u128)>;

    async fn defuse_mt_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<u128>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)>;

    async fn defuse_nft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_id: String,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)>;
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

impl DefuseExt for Near {
    async fn defuse_add_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::add_public_key(PublicKeyArgs {
                public_key: public_key.into(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_remove_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::remove_public_key(PublicKeyArgs {
                public_key: public_key.into(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_disable_auth_by_predecessor_id(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::disable_auth_by_predecessor_id(),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_set_fee(
        &self,
        defuse: impl Into<AccountId>,
        fee: Pips,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::set_fee(FeeArgs { fee }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_set_fee_collector(
        &self,
        defuse: impl Into<AccountId>,
        fee_collector: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::set_fee_collector(FeeCollectorArgs {
                fee_collector: &fee_collector.into(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_acl_grant_role(
        &self,
        defuse: impl Into<AccountId>,
        role: Role,
        account_id: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::acl_grant_role(AclRoleArgs {
                role,
                account_id: &account_id.into(),
            }),
            NearToken::from_near(0),
        )
        .await
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
        self.fn_call(
            defuse,
            Defuse::execute_intents(MultiPayloadArgs {
                signed: signed.into_iter().collect(),
            }),
            NearToken::from_near(0),
        )
        .await
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
        public_key: impl Into<PublicKey>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::add_relayer_key(PublicKeyArgs {
                public_key: public_key.into(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_delete_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::delete_relayer_key(PublicKeyArgs {
                public_key: public_key.into(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_cleanup_nonces(
        &self,
        defuse: impl Into<AccountId>,
        nonces: impl IntoIterator<Item = (AccountId, Vec<Nonce>)>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::cleanup_nonces(CleanupNoncesArgs {
                nonces: nonces
                    .into_iter()
                    .map(|(account_id, ns)| (account_id, ns.into_iter().map(Into::into).collect()))
                    .collect(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_force_add_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: HashMap<AccountId, HashSet<PublicKey>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_add_public_keys(ForcePublicKeysArgs { public_keys }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_force_remove_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: HashMap<AccountId, HashSet<PublicKey>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_remove_public_keys(ForcePublicKeysArgs { public_keys }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_force_lock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl Into<AccountId>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::force_lock_account(AccountArgs {
                    account_id: &account_id.into(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;
        let locked = res.json::<bool>()?;

        Ok((res.try_into()?, locked))
    }

    async fn defuse_force_unlock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl Into<AccountId>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::force_unlock_account(AccountArgs {
                    account_id: &account_id.into(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;
        let unlocked = res.json::<bool>()?;

        Ok((res.try_into()?, unlocked))
    }

    async fn defuse_force_enable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_enable_auth_by_predecessor_ids(MultipleAccountsArgs {
                account_ids: account_ids.into_iter().collect(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_force_disable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_disable_auth_by_predecessor_ids(MultipleAccountsArgs {
                account_ids: account_ids.into_iter().collect(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_ft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, u128)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::ft_withdraw(FtWithdrawArgs {
                    token: token.into(),
                    receiver_id: receiver_id.into(),
                    amount: amount.into(),
                    memo,
                    msg,
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;
        let transferred = res.json::<U128>()?.0;
        Ok((res.try_into()?, transferred))
    }

    async fn defuse_mt_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<u128>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::mt_withdraw(MtWithdrawArgs {
                    token: token.into(),
                    receiver_id: receiver_id.into(),
                    token_ids,
                    amounts: amounts.into_iter().map(U128::from).collect(),
                    memo,
                    msg,
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;
        let transferred = res.json::<Vec<U128>>()?.into_iter().map(|x| x.0).collect();
        Ok((res.try_into()?, transferred))
    }

    async fn defuse_nft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_id: String,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::nft_withdraw(NftWithdrawArgs {
                    token: token.into(),
                    receiver_id: receiver_id.into(),
                    token_id,
                    memo,
                    msg,
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;
        let transferred = res.json::<bool>()?;
        Ok((res.try_into()?, transferred))
    }
}
