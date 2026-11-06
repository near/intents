mod event;
#[cfg(feature = "imt")]
mod imt;
mod nonce;
mod signer;

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use defuse::{
    contract::config::DefuseConfig, near_promise::NearPromise, simulation_output::SimulationOutput,
};
use defuse_core::{
    Nonce, PublicKey, Salt, fees::Pips, intents::auth::AuthCall, payload::multi::MultiPayload,
};
pub use defuse_serde_utils::base64::AsBase64;
use near_kit::{
    AccountId, AccountIdRef, Gas, NearToken, SigningNear, rpc::FinalExecutionOutcome,
    transaction::FunctionCall,
};
use near_sdk::json_types::U128;
use serde_json::json;

use crate::{account::Account, extensions::FnCallTransaction, outcome::SuccessfulExecutionOutcome};

pub use event::*;
#[cfg(feature = "imt")]
pub use imt::*;
pub use nonce::*;
pub use signer::*;

pub use defuse::{contract, core, near_promise, tokens};
pub use defuse_nep245 as nep245;

#[near_kit::contract]
pub trait Defuse {
    fn fee(&self) -> Pips;
    fn fee_collector(&self) -> AccountId;

    fn has_public_key(&self, account_id: &AccountIdRef, public_key: &PublicKey) -> bool;
    fn public_keys_of(&self, account_id: &AccountIdRef) -> HashSet<PublicKey>;

    fn is_nonce_used(&self, account_id: &AccountIdRef, nonce: AsBase64<Nonce>) -> bool;
    fn is_auth_by_predecessor_id_enabled(&self, account_id: &AccountIdRef) -> bool;

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn add_public_key(&mut self, public_key: PublicKey);
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn remove_public_key(&mut self, public_key: PublicKey);

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn disable_auth_by_predecessor_id(&mut self);

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn set_fee(&mut self, fee: Pips);
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn set_fee_collector(&mut self, fee_collector: &AccountIdRef);

    fn current_salt(&self) -> Salt;
    fn is_valid_salt(&self, salt: Salt) -> bool;

    /// Returns `Salt`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn update_current_salt(&mut self);
    /// Returns `Salt`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn invalidate_salts(&mut self, salts: &[Salt]);

    #[call]
    fn admin_call(&mut self, promises: &[NearPromise]);

    fn simulate_intents(&self, signed: &[MultiPayload]) -> SimulationOutput;

    #[call]
    fn execute_intents(&mut self, signed: &[MultiPayload]);

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn add_relayer_key(&mut self, public_key: PublicKey);
    #[call]
    fn do_add_relayer_key(&mut self, public_key: PublicKey);
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn delete_relayer_key(&mut self, public_key: PublicKey);

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn cleanup_nonces(&mut self, nonces: &[(AccountId, Vec<AsBase64<Nonce>>)]);

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_add_public_keys(&mut self, public_keys: HashMap<AccountId, HashSet<PublicKey>>);
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_remove_public_keys(&mut self, public_keys: HashMap<AccountId, HashSet<PublicKey>>);

    fn is_account_locked(&self, account_id: &AccountIdRef) -> bool;

    /// Returns `bool`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_lock_account(&mut self, account_id: &AccountIdRef);
    /// Returns `bool`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_unlock_account(&mut self, account_id: &AccountIdRef);
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_disable_auth_by_predecessor_ids(&mut self, account_ids: &[AccountId]);
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_enable_auth_by_predecessor_ids(&mut self, account_ids: &[AccountId]);

    /// Returns `U128`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn ft_withdraw(
        &mut self,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: U128,
        memo: Option<String>,
        msg: Option<String>,
    );
    /// Returns `U128`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn ft_force_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: U128,
        memo: Option<String>,
        msg: Option<String>,
    );
    /// Returns `Vec<U128>`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn mt_withdraw(
        &mut self,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_ids: &[String],
        amounts: &[U128],
        memo: Option<String>,
        msg: Option<String>,
    );
    /// Returns `bool`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn nft_withdraw(
        &mut self,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: &str,
        memo: Option<String>,
        msg: Option<String>,
    );

    // NOTE: private method for testing purposes, not part of the public API
    #[call]
    fn do_auth_call(&mut self, signer_id: &AccountIdRef, auth_call: &AuthCall);
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

    async fn defuse_admin_call(
        &self,
        defuse: impl Into<AccountId>,
        promises: &[NearPromise],
        deposit: &NearToken,
        gas: Gas,
    ) -> Result<SuccessfulExecutionOutcome>;

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
        signer_id: impl AsRef<AccountIdRef>,
        auth_call: &AuthCall,
        gas: near_kit::Gas,
    ) -> Result<FinalExecutionOutcome>;
}

pub trait DefuseDeployerExt: Sized {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> Self;
}

impl DefuseDeployerExt for SigningNear {
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
            Some(
                FunctionCall::new("new")
                    .args(json!({"config": config}))
                    .gas(Gas::from_tgas(30)),
            ),
        )
        .await
        .unwrap()
    }
}

impl DefuseExt for SigningNear {
    async fn defuse_add_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::add_public_key(public_key.into()).gas(Gas::from_tgas(30)),
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
            Defuse::remove_public_key(public_key.into()).gas(Gas::from_tgas(30)),
        )
        .await
    }

    async fn defuse_disable_auth_by_predecessor_id(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::disable_auth_by_predecessor_id().gas(Gas::from_tgas(30)),
        )
        .await
    }

    async fn defuse_set_fee(
        &self,
        defuse: impl Into<AccountId>,
        fee: Pips,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(defuse, Defuse::set_fee(fee).gas(Gas::from_tgas(30)))
            .await
    }

    async fn defuse_set_fee_collector(
        &self,
        defuse: impl Into<AccountId>,
        fee_collector: impl AsRef<AccountIdRef>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::set_fee_collector(fee_collector.as_ref()).gas(Gas::from_tgas(30)),
        )
        .await
    }

    async fn defuse_update_current_salt(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<(SuccessfulExecutionOutcome, Salt)> {
        let outcome = self
            .transaction(defuse.into())
            .add_action(Defuse::update_current_salt().gas(Gas::from_tgas(30)))
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
                Defuse::invalidate_salts(&salts.into_iter().collect::<Vec<_>>())
                    .gas(Gas::from_tgas(300)),
            )
            .await?;
        let salt = outcome.json::<Salt>()?;
        Ok((outcome.try_into()?, salt))
    }

    async fn defuse_admin_call(
        &self,
        defuse: impl Into<AccountId>,
        promises: &[NearPromise],
        deposit: &NearToken,
        gas: Gas,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::admin_call(promises).deposit(*deposit).gas(gas),
        )
        .await
    }

    async fn defuse_execute_intents(
        &self,
        defuse: impl Into<AccountId>,
        signed: impl IntoIterator<Item = MultiPayload>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            defuse,
            Defuse::execute_intents(&signed.into_iter().collect::<Vec<_>>())
                .gas(Gas::from_tgas(300)),
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
            .simulate_intents(&signed)
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
            Defuse::add_relayer_key(*public_key).gas(Gas::from_tgas(30)),
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
            Defuse::delete_relayer_key(*public_key).gas(Gas::from_tgas(30)),
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
            Defuse::cleanup_nonces(
                &nonces
                    .into_iter()
                    .map(|(account_id, ns)| (account_id, ns.into_iter().map(AsBase64).collect()))
                    .collect::<Vec<_>>(),
            )
            .gas(Gas::from_tgas(300)),
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
            Defuse::force_add_public_keys(
                public_keys
                    .into_iter()
                    .map(|(account_id, public_keys)| {
                        (account_id, public_keys.into_iter().collect())
                    })
                    .collect(),
            )
            .gas(Gas::from_tgas(300)),
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
            Defuse::force_remove_public_keys(
                public_keys
                    .into_iter()
                    .map(|(account_id, public_keys)| {
                        (account_id, public_keys.into_iter().collect())
                    })
                    .collect(),
            )
            .gas(Gas::from_tgas(300)),
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
            .add_action(Defuse::force_lock_account(account_id.as_ref()).gas(Gas::from_tgas(30)))
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
            .add_action(Defuse::force_unlock_account(account_id.as_ref()).gas(Gas::from_tgas(30)))
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
            Defuse::force_enable_auth_by_predecessor_ids(
                &account_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
            )
            .gas(Gas::from_tgas(300)),
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
            Defuse::force_disable_auth_by_predecessor_ids(
                &account_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
            )
            .gas(Gas::from_tgas(300)),
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
                Defuse::ft_withdraw(
                    token.as_ref(),
                    receiver_id.as_ref(),
                    U128(amount),
                    memo,
                    msg,
                )
                .gas(Gas::from_tgas(300)),
            )
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
                Defuse::ft_force_withdraw(
                    owner_id.as_ref(),
                    token.as_ref(),
                    receiver_id.as_ref(),
                    U128(amount),
                    memo,
                    msg,
                )
                .gas(Gas::from_tgas(300)),
            )
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
                Defuse::mt_withdraw(
                    token.as_ref(),
                    receiver_id.as_ref(),
                    &token_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
                    &amounts.into_iter().map(U128).collect::<Vec<_>>(),
                    memo,
                    msg,
                )
                .gas(Gas::from_tgas(300)),
            )
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
                Defuse::nft_withdraw(
                    token.as_ref(),
                    receiver_id.as_ref(),
                    token_id.as_ref(),
                    memo,
                    msg,
                )
                .gas(Gas::from_tgas(300)),
            )
            .await?;
        let transferred = res.json::<bool>()?;
        Ok((res.try_into()?, transferred))
    }

    async fn defuse_do_auth_call(
        &self,
        defuse: impl Into<AccountId>,
        signer_id: impl AsRef<AccountIdRef>,
        auth_call: &AuthCall,
        gas: Gas,
    ) -> Result<FinalExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(Defuse::do_auth_call(signer_id.as_ref(), auth_call).gas(gas))
            .await
            .map_err(Into::into)
    }
}
