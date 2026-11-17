use anyhow::Result;
use defuse_poa_factory::contract::Role;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_kit::{
    AccountId, AccountIdRef, Gas, NearToken, SigningNear,
    json_types::U128,
    rpc::CheckedOutcome,
    signer::PublicKey,
    standards::{FungibleToken, FungibleTokenClient},
    transaction::FunctionCall,
};
use serde_json::json;
use std::collections::{HashMap, HashSet};

use crate::account::AccountExt;

pub use defuse_poa_factory::{IdDigest, PayloadHash, Withdrawal, contract};

pub const POA_TOKEN_INIT_BALANCE: NearToken = NearToken::from_near(3);

#[near_kit::contract]
pub trait PoaFactory {
    /// See [`deploy_token()`](defuse_poa_factory::PoaFactory::deploy_token) method.
    #[call]
    fn deploy_token(&mut self, token: &str, metadata: Option<&FungibleTokenMetadata>);

    /// See [`set_metadata()`](defuse_poa_factory::PoaFactory::set_metadata) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn set_metadata(&mut self, token: &str, metadata: &FungibleTokenMetadata);

    /// See [`ft_deposit()`](defuse_poa_factory::PoaFactory::ft_deposit) method.
    #[call]
    fn ft_deposit(
        &mut self,
        token: &str,
        owner_id: &AccountIdRef,
        amount: U128,
        msg: Option<&str>,
        memo: Option<&str>,
    );

    /// See [`record_withdrawal()`](defuse_poa_factory::PoaFactory::record_withdrawal) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn record_withdrawal(&mut self, withdrawal_id: IdDigest, withdrawal: &Withdrawal);

    /// See [`update_withdrawal()`](defuse_poa_factory::PoaFactory::update_withdrawal) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn update_withdrawal(
        &mut self,
        withdrawal_id: IdDigest,
        prev_payload_hash: PayloadHash,
        new_payload_hash: PayloadHash,
        metadata: &str,
    );

    /// See [`remove_withdrawals()`](defuse_poa_factory::PoaFactory::remove_withdrawals) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn remove_withdrawals(&mut self, withdrawal_ids: &[IdDigest]);

    /// See [`remove_deposits()`](defuse_poa_factory::PoaFactory::remove_deposits) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn remove_deposits(&mut self, deposit_ids: &[IdDigest]);

    /// See [`get_withdrawal()`](defuse_poa_factory::PoaFactory::get_withdrawal) method.
    fn get_withdrawal(&self, withdrawal_id: IdDigest) -> Option<Withdrawal>;

    /// See [`add_full_access_key_to_tokens()`](defuse_poa_factory::PoaFactory::add_full_access_key_to_tokens) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn add_full_access_key_to_tokens(&mut self, public_key: &PublicKey, tokens: &[String]);

    /// See [`delete_key_from_tokens()`](defuse_poa_factory::PoaFactory::delete_key_from_tokens) method.
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn delete_key_from_tokens(&mut self, public_key: &PublicKey, tokens: &[String]);

    /// See [`tokens()`](defuse_poa_factory::PoaFactory::tokens) method.
    fn tokens(&self) -> HashMap<String, AccountId>;
}

impl<C> PoaFactoryClient<C> {
    pub fn ft_name(&self, ft: &AccountIdRef) -> String {
        ft.as_str()
            .trim_end_matches(&format!(".{}", self.contract_id()))
            .to_string()
    }
}

pub trait PoaFactoryDeployerExt: Sized {
    async fn deploy_poa_factory(
        &self,
        name: impl AsRef<str>,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        wasm: impl Into<Vec<u8>>,
    ) -> PoaFactoryClient<Self>;
}

impl PoaFactoryDeployerExt for SigningNear {
    async fn deploy_poa_factory(
        &self,
        name: impl AsRef<str>,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        wasm: impl Into<Vec<u8>>,
    ) -> PoaFactoryClient<Self> {
        let action = FunctionCall::new("new")
            .args(json!({
                "super_admins": super_admins.into_iter().collect::<HashSet<_>>(),
                "admins": admins
                    .into_iter()
                    .map(|(role, admins)| (role, admins.into_iter().collect::<HashSet<_>>()))
                    .collect::<HashMap<_, _>>(),
                "grantees": grantees
                    .into_iter()
                    .map(|(role, grantees)| (role, grantees.into_iter().collect::<HashSet<_>>()))
                    .collect::<HashMap<_, _>>(),
            }))
            .gas(Gas::from_tgas(30));

        let account = self
            .deploy_sub_contract(name, NearToken::from_near(10), wasm, Some(action))
            .await
            .unwrap();

        self.contract::<PoaFactory>(account.account_id())
    }
}

pub trait PoAFactoryExt: Sized {
    async fn poa_factory_deploy_token(
        &self,
        factory: impl AsRef<AccountIdRef>,
        token: impl AsRef<str>,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> Result<FungibleTokenClient<Self>>;

    async fn poa_factory_ft_deposit(
        &self,
        factory: impl AsRef<AccountIdRef>,
        token: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> Result<CheckedOutcome>;

    async fn poa_factory_record_withdrawal(
        &self,
        factory: impl AsRef<AccountIdRef>,
        withdrawal_id: IdDigest,
        withdrawal: Withdrawal,
    ) -> Result<CheckedOutcome>;

    async fn poa_factory_update_withdrawal(
        &self,
        factory: impl AsRef<AccountIdRef>,
        withdrawal_id: IdDigest,
        prev_payload_hash: PayloadHash,
        new_payload_hash: PayloadHash,
        metadata: impl Into<String>,
    ) -> Result<CheckedOutcome>;

    async fn poa_factory_remove_withdrawals(
        &self,
        factory: impl AsRef<AccountIdRef>,
        withdrawal_ids: Vec<IdDigest>,
    ) -> Result<CheckedOutcome>;

    async fn poa_factory_remove_deposits(
        &self,
        factory: impl AsRef<AccountIdRef>,
        deposit_ids: Vec<IdDigest>,
    ) -> Result<CheckedOutcome>;
}

impl PoAFactoryExt for SigningNear {
    async fn poa_factory_deploy_token(
        &self,
        factory: impl AsRef<AccountIdRef>,
        token: impl AsRef<str>,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> Result<FungibleTokenClient<Self>> {
        let factory = factory.as_ref();
        self.transaction(factory)
            .add_action(
                PoaFactory::deploy_token(token.as_ref(), metadata.into().as_ref())
                    .deposit(POA_TOKEN_INIT_BALANCE)
                    .gas(Gas::from_tgas(300)),
            )
            .await?
            .checked()?;

        Ok(self.contract::<FungibleToken>(factory.sub_account(token)?))
    }

    async fn poa_factory_ft_deposit(
        &self,
        factory: impl AsRef<AccountIdRef>,
        token: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> Result<CheckedOutcome> {
        self.transaction(factory.as_ref())
            .add_action(
                PoaFactory::ft_deposit(
                    token.as_ref(),
                    owner_id.as_ref(),
                    U128(amount),
                    msg.as_deref(),
                    memo.as_deref(),
                )
                .deposit(NearToken::from_millinear(4))
                .gas(Gas::from_tgas(300)),
            )
            .await?
            .checked()
            .map_err(Into::into)
    }

    async fn poa_factory_record_withdrawal(
        &self,
        factory: impl AsRef<AccountIdRef>,
        withdrawal_id: IdDigest,
        withdrawal: Withdrawal,
    ) -> Result<CheckedOutcome> {
        self.transaction(factory.as_ref())
            .add_action(
                PoaFactory::record_withdrawal(withdrawal_id, &withdrawal).gas(Gas::from_tgas(30)),
            )
            .await?
            .checked()
            .map_err(Into::into)
    }

    async fn poa_factory_update_withdrawal(
        &self,
        factory: impl AsRef<AccountIdRef>,
        withdrawal_id: IdDigest,
        prev_payload_hash: PayloadHash,
        new_payload_hash: PayloadHash,
        metadata: impl Into<String>,
    ) -> Result<CheckedOutcome> {
        self.transaction(factory.as_ref())
            .add_action(
                PoaFactory::update_withdrawal(
                    withdrawal_id,
                    prev_payload_hash,
                    new_payload_hash,
                    &metadata.into(),
                )
                .deposit(NearToken::from_yoctonear(4))
                .gas(Gas::from_tgas(30)),
            )
            .await?
            .checked()
            .map_err(Into::into)
    }

    async fn poa_factory_remove_withdrawals(
        &self,
        factory: impl AsRef<AccountIdRef>,
        withdrawal_ids: Vec<IdDigest>,
    ) -> Result<CheckedOutcome> {
        self.transaction(factory.as_ref())
            .add_action(PoaFactory::remove_withdrawals(&withdrawal_ids).gas(Gas::from_tgas(30)))
            .await?
            .checked()
            .map_err(Into::into)
    }

    async fn poa_factory_remove_deposits(
        &self,
        factory: impl AsRef<AccountIdRef>,
        deposit_ids: Vec<IdDigest>,
    ) -> Result<CheckedOutcome> {
        self.transaction(factory.as_ref())
            .add_action(PoaFactory::remove_deposits(&deposit_ids).gas(Gas::from_tgas(30)))
            .await?
            .checked()
            .map_err(Into::into)
    }
}
