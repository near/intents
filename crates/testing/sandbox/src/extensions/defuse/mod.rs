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
    AccountId, AccountIdRef, Gas, NearToken, SigningNear,
    json_types::U128,
    rpc::{CheckedOutcome, FinalExecutionOutcome},
    transaction::FunctionCall,
};
use serde_json::json;

use crate::{account::AccountExt, extensions::FnCallTransaction};

pub use event::*;
#[cfg(feature = "imt")]
pub use imt::*;
pub use nonce::*;
pub use signer::*;

pub use defuse::{contract, core, near_promise, tokens};
pub use defuse_nep245 as nep245;

#[near_kit::contract]
pub trait Defuse {
    /// See [`fee()`](defuse::fees::FeesManager::fee) method.
    fn fee(&self) -> Pips;
    /// See [`fee_collector()`](defuse::fees::FeesManager::fee_collector) method.
    fn fee_collector(&self) -> AccountId;

    /// See [`has_public_key()`](defuse::accounts::AccountManager::has_public_key) method.
    fn has_public_key(&self, account_id: &AccountIdRef, public_key: &PublicKey) -> bool;
    /// See [`public_keys_of()`](defuse::accounts::AccountManager::public_keys_of) method.
    fn public_keys_of(&self, account_id: &AccountIdRef) -> HashSet<PublicKey>;

    /// See [`is_nonce_used()`](defuse::accounts::AccountManager::is_nonce_used) method.
    fn is_nonce_used(&self, account_id: &AccountIdRef, nonce: AsBase64<Nonce>) -> bool;
    /// See [`is_auth_by_predecessor_id_enabled()`](defuse::accounts::AccountManager::is_auth_by_predecessor_id_enabled) method.
    fn is_auth_by_predecessor_id_enabled(&self, account_id: &AccountIdRef) -> bool;

    /// See [`add_public_key()`](defuse::accounts::AccountManager::add_public_key) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn add_public_key(&mut self, public_key: PublicKey);
    /// See [`remove_public_key()`](defuse::accounts::AccountManager::remove_public_key) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn remove_public_key(&mut self, public_key: PublicKey);

    /// See [`disable_auth_by_predecessor_id()`](defuse::accounts::AccountManager::disable_auth_by_predecessor_id) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn disable_auth_by_predecessor_id(&mut self);

    /// See [`set_fee()`](defuse::fees::FeesManager::set_fee) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn set_fee(&mut self, fee: Pips);
    /// See [`set_fee_collector()`](defuse::fees::FeesManager::set_fee_collector) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn set_fee_collector(&mut self, fee_collector: &AccountIdRef);

    /// See [`current_salt()`](defuse::salts::SaltManager::current_salt) method.
    fn current_salt(&self) -> Salt;
    /// See [`is_valid_salt()`](defuse::salts::SaltManager::is_valid_salt) method.
    fn is_valid_salt(&self, salt: Salt) -> bool;

    /// See [`update_current_salt()`](defuse::salts::SaltManager::update_current_salt) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn update_current_salt(&mut self);
    /// See [`invalidate_salts()`](defuse::salts::SaltManager::invalidate_salts) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn invalidate_salts(&mut self, salts: &[Salt]);

    /// See [`admin_call()`](defuse::admin_call::AdminCallManager::admin_call) method.
    #[call]
    fn admin_call(&mut self, promises: &[NearPromise]);

    /// See [`simulate_intents()`](defuse::intents::Intents::simulate_intents) method.
    fn simulate_intents(&self, signed: &[MultiPayload]) -> SimulationOutput;

    /// See [`execute_intents()`](defuse::intents::Intents::execute_intents) method.
    #[call]
    fn execute_intents(&mut self, signed: &[MultiPayload]);

    /// See [`add_relayer_key()`](defuse::intents::RelayerKeys::add_relayer_key) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn add_relayer_key(&mut self, public_key: PublicKey);
    /// See [`do_add_relayer_key()`](defuse::intents::RelayerKeys::do_add_relayer_key) method.
    #[call]
    fn do_add_relayer_key(&mut self, public_key: PublicKey);
    /// See [`delete_relayer_key()`](defuse::intents::RelayerKeys::delete_relayer_key) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn delete_relayer_key(&mut self, public_key: PublicKey);

    /// See [`cleanup_nonces()`](defuse::garbage_collector::GarbageCollector::cleanup_nonces) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn cleanup_nonces(&mut self, nonces: &[(AccountId, Vec<AsBase64<Nonce>>)]);

    /// See [`force_add_public_keys()`](defuse::far::ForcePublicKeyManager::force_add_public_keys) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_add_public_keys(&mut self, public_keys: &HashMap<AccountId, HashSet<PublicKey>>);
    /// See [`force_remove_public_keys()`](defuse::far::ForcePublicKeyManager::force_remove_public_keys) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_remove_public_keys(&mut self, public_keys: &HashMap<AccountId, HashSet<PublicKey>>);

    /// See [`is_account_locked()`](defuse::accounts::ForceAccountManager::is_account_locked) method.
    fn is_account_locked(&self, account_id: &AccountIdRef) -> bool;

    /// See [`force_lock_account()`](defuse::accounts::ForceAccountManager::force_lock_account) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_lock_account(&mut self, account_id: &AccountIdRef);
    /// See [`force_unlock_account()`](defuse::accounts::ForceAccountManager::force_unlock_account) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_unlock_account(&mut self, account_id: &AccountIdRef);
    /// See [`force_disable_auth_by_predecessor_ids()`](defuse::accounts::ForceAccountManager::force_disable_auth_by_predecessor_ids) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_disable_auth_by_predecessor_ids(&mut self, account_ids: &[AccountId]);
    /// See [`force_enable_auth_by_predecessor_ids()`](defuse::accounts::ForceAccountManager::force_enable_auth_by_predecessor_ids) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn force_enable_auth_by_predecessor_ids(&mut self, account_ids: &[AccountId]);

    /// See [`ft_withdraw()`](defuse::tokens::nep141::FungibleTokenWithdrawer::ft_withdraw) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn ft_withdraw(
        &mut self,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: U128,
        memo: Option<&str>,
        msg: Option<&str>,
    );
    /// See [`ft_force_withdraw()`](defuse::tokens::nep141::FungibleTokenForceWithdrawer::ft_force_withdraw) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn ft_force_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: U128,
        memo: Option<&str>,
        msg: Option<&str>,
    );
    /// See [`mt_withdraw()`](defuse::tokens::nep245::MultiTokenWithdrawer::mt_withdraw) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn mt_withdraw(
        &mut self,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_ids: &[String],
        amounts: &[U128],
        memo: Option<&str>,
        msg: Option<&str>,
    );
    /// See [`nft_withdraw()`](defuse::tokens::nep171::NonFungibleTokenWithdrawer::nft_withdraw) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn nft_withdraw(
        &mut self,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: &str,
        memo: Option<&str>,
        msg: Option<&str>,
    );

    // NOTE: private method for testing purposes, not part of the public API
    /// See [`do_auth_call()`](defuse::contract::Contract::do_auth_call) method.
    #[call]
    fn do_auth_call(&mut self, signer_id: &AccountIdRef, auth_call: &AuthCall);
}

pub trait DefuseExt {
    async fn defuse_add_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_remove_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_disable_auth_by_predecessor_id(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_set_fee(
        &self,
        defuse: impl Into<AccountId>,
        fee: Pips,
    ) -> Result<CheckedOutcome>;

    async fn defuse_set_fee_collector(
        &self,
        defuse: impl Into<AccountId>,
        fee_collector: impl AsRef<AccountIdRef>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_update_current_salt(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<(CheckedOutcome, Salt)>;

    async fn defuse_invalidate_salts(
        &self,
        defuse: impl Into<AccountId>,
        salts: impl IntoIterator<Item = Salt>,
    ) -> Result<(CheckedOutcome, Salt)>;

    async fn defuse_admin_call(
        &self,
        defuse: impl Into<AccountId>,
        promises: &[NearPromise],
        deposit: &NearToken,
        gas: Gas,
    ) -> Result<CheckedOutcome>;

    async fn defuse_execute_intents(
        &self,
        defuse: impl Into<AccountId>,
        signed: impl IntoIterator<Item = MultiPayload>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_simulate_and_execute_intents(
        &self,
        defuse: impl Into<AccountId>,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> Result<(CheckedOutcome, SimulationOutput)>;

    async fn defuse_add_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> Result<CheckedOutcome>;

    async fn defuse_delete_relayer_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: &PublicKey,
    ) -> Result<CheckedOutcome>;

    async fn defuse_cleanup_nonces(
        &self,
        defuse: impl Into<AccountId>,
        nonces: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = Nonce>)>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_force_add_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = PublicKey>)>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_force_remove_public_keys(
        &self,
        defuse: impl Into<AccountId>,
        public_keys: impl IntoIterator<Item = (AccountId, impl IntoIterator<Item = PublicKey>)>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_force_lock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<(CheckedOutcome, bool)>;

    async fn defuse_force_unlock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<(CheckedOutcome, bool)>;

    async fn defuse_force_disable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item: Into<AccountId>>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_force_enable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item: Into<AccountId>>,
    ) -> Result<CheckedOutcome>;

    async fn defuse_ft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(CheckedOutcome, u128)>;

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
    ) -> Result<(CheckedOutcome, u128)>;

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
    ) -> Result<(CheckedOutcome, Vec<u128>)>;

    async fn defuse_nft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(CheckedOutcome, bool)>;

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
    ) -> Result<CheckedOutcome> {
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
    ) -> Result<CheckedOutcome> {
        self.fn_call(
            defuse,
            Defuse::remove_public_key(public_key.into()).gas(Gas::from_tgas(30)),
        )
        .await
    }

    async fn defuse_disable_auth_by_predecessor_id(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<CheckedOutcome> {
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
    ) -> Result<CheckedOutcome> {
        self.fn_call(defuse, Defuse::set_fee(fee).gas(Gas::from_tgas(30)))
            .await
    }

    async fn defuse_set_fee_collector(
        &self,
        defuse: impl Into<AccountId>,
        fee_collector: impl AsRef<AccountIdRef>,
    ) -> Result<CheckedOutcome> {
        self.fn_call(
            defuse,
            Defuse::set_fee_collector(fee_collector.as_ref()).gas(Gas::from_tgas(30)),
        )
        .await
    }

    async fn defuse_update_current_salt(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<(CheckedOutcome, Salt)> {
        self.fn_call_json(
            defuse,
            Defuse::update_current_salt().gas(Gas::from_tgas(30)),
        )
        .await
    }

    async fn defuse_invalidate_salts(
        &self,
        defuse: impl Into<AccountId>,
        salts: impl IntoIterator<Item = Salt>,
    ) -> Result<(CheckedOutcome, Salt)> {
        self.fn_call_json(
            defuse,
            Defuse::invalidate_salts(&salts.into_iter().collect::<Vec<_>>())
                .gas(Gas::from_tgas(300)),
        )
        .await
    }

    async fn defuse_admin_call(
        &self,
        defuse: impl Into<AccountId>,
        promises: &[NearPromise],
        deposit: &NearToken,
        gas: Gas,
    ) -> Result<CheckedOutcome> {
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
    ) -> Result<CheckedOutcome> {
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
    ) -> Result<(CheckedOutcome, SimulationOutput)> {
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
    ) -> Result<CheckedOutcome> {
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
    ) -> Result<CheckedOutcome> {
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
    ) -> Result<CheckedOutcome> {
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
    ) -> Result<CheckedOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_add_public_keys(
                &public_keys
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
    ) -> Result<CheckedOutcome> {
        self.fn_call(
            defuse,
            Defuse::force_remove_public_keys(
                &public_keys
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
    ) -> Result<(CheckedOutcome, bool)> {
        self.fn_call_json(
            defuse,
            Defuse::force_lock_account(account_id.as_ref()).gas(Gas::from_tgas(30)),
        )
        .await
    }

    async fn defuse_force_unlock_account(
        &self,
        defuse: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<(CheckedOutcome, bool)> {
        self.fn_call_json(
            defuse,
            Defuse::force_unlock_account(account_id.as_ref()).gas(Gas::from_tgas(30)),
        )
        .await
    }

    async fn defuse_force_enable_auth_by_predecessor_ids(
        &self,
        defuse: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item: Into<AccountId>>,
    ) -> Result<CheckedOutcome> {
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
    ) -> Result<CheckedOutcome> {
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
    ) -> Result<(CheckedOutcome, u128)> {
        let (outcome, U128(transferred)) = self
            .fn_call_json(
                defuse,
                Defuse::ft_withdraw(
                    token.as_ref(),
                    receiver_id.as_ref(),
                    U128(amount),
                    memo.as_deref(),
                    msg.as_deref(),
                )
                .gas(Gas::from_tgas(300)),
            )
            .await?;
        Ok((outcome, transferred))
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
    ) -> Result<(CheckedOutcome, u128)> {
        let (outcome, U128(transferred)) = self
            .fn_call_json(
                defuse,
                Defuse::ft_force_withdraw(
                    owner_id.as_ref(),
                    token.as_ref(),
                    receiver_id.as_ref(),
                    U128(amount),
                    memo.as_deref(),
                    msg.as_deref(),
                )
                .gas(Gas::from_tgas(300)),
            )
            .await?;
        Ok((outcome, transferred))
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
    ) -> Result<(CheckedOutcome, Vec<u128>)> {
        let (outcome, transferred): (_, Vec<U128>) = self
            .fn_call_json(
                defuse,
                Defuse::mt_withdraw(
                    token.as_ref(),
                    receiver_id.as_ref(),
                    &token_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
                    &amounts.into_iter().map(U128).collect::<Vec<_>>(),
                    memo.as_deref(),
                    msg.as_deref(),
                )
                .gas(Gas::from_tgas(300)),
            )
            .await?;
        Ok((outcome, transferred.into_iter().map(|x| x.0).collect()))
    }

    async fn defuse_nft_withdraw(
        &self,
        defuse: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Result<(CheckedOutcome, bool)> {
        self.fn_call_json(
            defuse,
            Defuse::nft_withdraw(
                token.as_ref(),
                receiver_id.as_ref(),
                token_id.as_ref(),
                memo.as_deref(),
                msg.as_deref(),
            )
            .gas(Gas::from_tgas(300)),
        )
        .await
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
