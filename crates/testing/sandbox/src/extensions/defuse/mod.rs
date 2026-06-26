mod event;
#[cfg(feature = "imt")]
mod imt;
mod nonce;
mod signer;

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use defuse::{contract::config::DefuseConfig, simulation_output::SimulationOutput};
use defuse_core::{
    Nonce, PublicKey, Salt, fees::Pips, intents::auth::AuthCall, payload::multi::MultiPayload,
};
use near_kit::{
    AccountId, AccountIdRef, Final, FinalExecutionOutcome, FunctionCallAction, Near, NearToken,
};
use near_sdk::json_types::U128;
use serde::Serialize;
use serde_json::json;
use serde_with::{DisplayFromStr, base64::Base64, serde_as};

use crate::{
    account::Account,
    extensions::{DEFAULT_GAS, FnCallTransaction},
    outcome::SuccessfulExecutionOutcome,
};

pub use event::*;
#[cfg(feature = "imt")]
pub use imt::*;
pub use nonce::*;
pub use signer::*;

pub use defuse::contract;
pub use defuse::core;
pub use defuse::tokens;
pub use defuse_nep245 as nep245;

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
pub struct InvalidateSaltArgs<'a> {
    pub salts: &'a [Salt],
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
pub struct MultiPayloadArgs<'a> {
    pub signed: &'a [MultiPayload],
}

#[serde_as]
#[derive(Serialize)]
pub struct CleanupNoncesArgs<'a> {
    #[serde_as(as = "&[(_, Vec<Base64>)]")]
    pub nonces: &'a [(AccountId, Vec<Nonce>)],
}

#[derive(Serialize)]
pub struct ForcePublicKeysArgs {
    pub public_keys: HashMap<AccountId, HashSet<PublicKey>>,
}

#[derive(Serialize)]
pub struct AccountArgs<'a> {
    pub account_id: &'a AccountIdRef,
}

#[derive(Serialize)]
pub struct MultipleAccountsArgs<'a> {
    pub account_ids: &'a [AccountId],
}

#[serde_as]
#[derive(Serialize)]
pub struct FtWithdrawArgs<'a> {
    pub token: &'a AccountIdRef,
    pub receiver_id: &'a AccountIdRef,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[serde_as]
#[derive(Serialize)]
pub struct FtForceWithdrawArgs<'a> {
    pub owner_id: &'a AccountIdRef,
    pub token: &'a AccountIdRef,
    pub receiver_id: &'a AccountIdRef,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[serde_as]
#[derive(Serialize)]
pub struct MtWithdrawArgs<'a> {
    pub token: &'a AccountIdRef,
    pub receiver_id: &'a AccountIdRef,
    pub token_ids: &'a [String],
    #[serde_as(as = "&[DisplayFromStr]")]
    pub amounts: &'a [u128],
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize)]
pub struct NftWithdrawArgs<'a> {
    pub token: &'a AccountIdRef,
    pub receiver_id: &'a AccountIdRef,
    pub token_id: &'a str,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize)]
pub struct DoAuthCallArgs<'a> {
    pub signer_id: &'a AccountIdRef,
    pub auth_call: &'a AuthCall,
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

    fn current_salt(&self) -> Salt;
    fn is_valid_salt(&self, salt: SaltArgs) -> bool;

    #[call]
    fn update_current_salt(&mut self) -> Salt;
    #[call]
    fn invalidate_salts(&mut self, args: InvalidateSaltArgs) -> Salt;

    fn simulate_intents(&self, args: MultiPayloadArgs) -> SimulationOutput;

    #[call]
    fn execute_intents(&mut self, args: MultiPayloadArgs);

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
    fn ft_force_withdraw(&mut self, args: FtForceWithdrawArgs) -> U128;
    #[call]
    fn mt_withdraw(&mut self, args: MtWithdrawArgs) -> Vec<U128>;
    #[call]
    fn nft_withdraw(&mut self, args: NftWithdrawArgs) -> bool;

    // NOTE: private method for testing purposes, not part of the public API
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
        fee_collector: impl AsRef<AccountIdRef>,
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
    ) -> Result<(SuccessfulExecutionOutcome, SimulationOutput)>;

    async fn defuse_add_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_delete_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_cleanup_nonces(
        &self,
        defuse: impl Into<AccountId>,
        nonces: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = Nonce>)>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_force_add_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = PublicKey>)>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_force_remove_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = PublicKey>)>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_force_lock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)>;

    async fn defuse_force_unlock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)>;

    async fn defuse_force_disable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item: Into<AccountId>>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_force_enable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item: Into<AccountId>>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_ft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, u128)>;

    #[allow(clippy::too_many_arguments)]
    async fn defuse_ft_force_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        owner_id: impl AsRef<AccountIdRef>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<u128>;

    #[allow(clippy::too_many_arguments)]
    async fn defuse_mt_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item: Into<String>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)>;

    async fn defuse_nft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)>;

    async fn defuse_do_auth_call(
        &self,
        defuse: impl Into<AccountId>,
        args: DoAuthCallArgs,
        gas: near_kit::Gas,
    ) -> Result<FinalExecutionOutcome>;
}

pub trait DefuseDeployerExt {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> Near;
}

impl DefuseDeployerExt for Near {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> Self {
        self.deploy_sub_contract(
            name,
            NearToken::from_near(100),
            wasm,
            Some(FunctionCallAction {
                method_name: "new".to_string(),
                args: json!({"config": config}).to_string().as_bytes().to_vec(),
                gas: DEFAULT_GAS,
                deposit: NearToken::from_near(0),
            }),
        )
        .await
        .unwrap()
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
        fee_collector: impl AsRef<AccountIdRef>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::set_fee_collector(FeeCollectorArgs {
                fee_collector: fee_collector.as_ref(),
            }),
            NearToken::from_yoctonear(1),
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
                    salts: &salts.into_iter().collect::<Vec<_>>(),
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
                signed: &signed.into_iter().collect::<Vec<_>>(),
            }),
            NearToken::from_near(0),
        )
        .await
    }

    async fn defuse_simulate_and_execute_intents(
        &self,
        defuse: impl Into<AccountId>,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> Result<(SuccessfulExecutionOutcome, SimulationOutput)> {
        let defuse_id = defuse.into();
        let signed: Vec<MultiPayload> = intents.into_iter().collect();

        let simulation_result = self
            .contract::<Defuse>(defuse_id.clone())
            .simulate_intents(MultiPayloadArgs { signed: &signed })
            .await?;

        Ok((
            self.defuse_execute_intents(defuse_id, signed).await?,
            simulation_result,
        ))
    }

    async fn defuse_add_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::add_relayer_key(PublicKeyArgs {
                public_key: *public_key,
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_delete_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::delete_relayer_key(PublicKeyArgs {
                public_key: *public_key,
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_cleanup_nonces(
        &self,
        defuse: impl Into<AccountId>,
        nonces: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = Nonce>)>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::cleanup_nonces(CleanupNoncesArgs {
                nonces: &nonces
                    .into_iter()
                    .map(|(account_id, ns)| (account_id, ns.into_iter().collect()))
                    .collect::<Vec<_>>(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_force_add_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = PublicKey>)>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_add_public_keys(ForcePublicKeysArgs {
                public_keys: public_keys
                    .into_iter()
                    .map(|(account_id, public_keys)| {
                        (account_id, public_keys.into_iter().collect())
                    })
                    .collect(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_force_remove_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = PublicKey>)>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_remove_public_keys(ForcePublicKeysArgs {
                public_keys: public_keys
                    .into_iter()
                    .map(|(account_id, public_keys)| {
                        (account_id, public_keys.into_iter().collect())
                    })
                    .collect(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_force_lock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::force_lock_account(AccountArgs {
                    account_id: account_id.as_ref(),
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
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::force_unlock_account(AccountArgs {
                    account_id: account_id.as_ref(),
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
        account_ids: impl IntoIterator<Item: Into<AccountId>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_enable_auth_by_predecessor_ids(MultipleAccountsArgs {
                account_ids: &account_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_force_disable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item: Into<AccountId>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_disable_auth_by_predecessor_ids(MultipleAccountsArgs {
                account_ids: &account_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
            }),
            NearToken::from_yoctonear(1),
        )
        .await
    }

    async fn defuse_ft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, u128)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::ft_withdraw(FtWithdrawArgs {
                    token: token.as_ref(),
                    receiver_id: receiver_id.as_ref(),
                    amount,
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

    async fn defuse_ft_force_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        owner_id: impl AsRef<AccountIdRef>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<u128> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::ft_force_withdraw(FtForceWithdrawArgs {
                    owner_id: owner_id.as_ref(),
                    token: token.as_ref(),
                    receiver_id: receiver_id.as_ref(),
                    amount,
                    memo,
                    msg,
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;
        Ok(res.json::<U128>()?.0)
    }

    async fn defuse_mt_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item: Into<String>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::mt_withdraw(MtWithdrawArgs {
                    token: token.as_ref(),
                    receiver_id: receiver_id.as_ref(),
                    token_ids: &token_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
                    amounts: &amounts.into_iter().collect::<Vec<_>>(),
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
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(SuccessfulExecutionOutcome, bool)> {
        let res = self
            .transaction(defuse.into())
            .add_action(
                Defuse::nft_withdraw(NftWithdrawArgs {
                    token: token.as_ref(),
                    receiver_id: receiver_id.as_ref(),
                    token_id: token_id.as_ref(),
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

    async fn defuse_do_auth_call(
        &self,
        defuse: impl Into<AccountId>,
        args: DoAuthCallArgs<'_>,
        gas: near_kit::Gas,
    ) -> Result<FinalExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(Defuse::do_auth_call(args).gas(gas))
            .wait_until(Final)
            .await
            .map_err(Into::into)
    }
}
