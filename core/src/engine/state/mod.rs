pub mod cached;
pub mod deltas;

use std::{borrow::Cow, iter};

use cached::CachedState;
use defuse_crypto::PublicKey;
use impl_tools::autoimpl;
use near_sdk::{AccountId, AccountIdRef};

use crate::{
    fees::Pips,
    intents::tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw},
    tokens::TokenId,
    DefuseError, Nonce, Result,
};

#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>)]
pub trait StateView {
    fn verifying_contract(&self) -> Cow<'_, AccountIdRef>;
    fn wnear_id(&self) -> Cow<'_, AccountIdRef>;

    fn fee(&self) -> Pips;
    fn fee_collector(&self) -> Cow<'_, AccountIdRef>;

    #[must_use]
    fn has_public_key(&self, account_id: &AccountIdRef, public_key: &PublicKey) -> bool;
    fn iter_public_keys(&self, account_id: &AccountIdRef) -> impl Iterator<Item = PublicKey> + '_;

    #[must_use]
    fn is_nonce_used(&self, account_id: &AccountIdRef, nonce: Nonce) -> bool;

    #[must_use]
    fn balance_of(&self, account_id: &AccountIdRef, token_id: &TokenId) -> u128;

    #[inline]
    fn cached(self) -> CachedState<Self>
    where
        Self: Sized,
    {
        CachedState::new(self)
    }
}

#[autoimpl(for<T: trait + ?Sized> &mut T, Box<T>)]
pub trait State: StateView {
    #[must_use]
    fn add_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> bool;

    #[must_use]
    fn remove_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> bool;

    #[must_use]
    fn commit_nonce(&mut self, account_id: AccountId, nonce: Nonce) -> bool;

    fn internal_add_balance(
        &mut self,
        owner_id: AccountId,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()>;

    fn internal_sub_balance(
        &mut self,
        owner_id: &AccountIdRef,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()>;

    fn internal_apply_deltas(
        &mut self,
        owner_id: &AccountIdRef,
        tokens: impl IntoIterator<Item = (TokenId, i128)>,
    ) -> Result<()> {
        for (token_id, delta) in tokens {
            let tokens = [(token_id, delta.unsigned_abs())];
            if delta.is_negative() {
                self.internal_sub_balance(owner_id, tokens)?;
            } else {
                self.internal_add_balance(owner_id.to_owned(), tokens)?;
            }
        }
        Ok(())
    }

    fn ft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: FtWithdraw) -> Result<()> {
        self.internal_sub_balance(
            owner_id,
            iter::once((TokenId::Nep141(withdraw.token.clone()), withdraw.amount.0)).chain(
                withdraw.storage_deposit.map(|amount| {
                    (
                        TokenId::Nep141(self.wnear_id().into_owned()),
                        amount.as_yoctonear(),
                    )
                }),
            ),
        )
    }

    fn nft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NftWithdraw) -> Result<()> {
        self.internal_sub_balance(
            owner_id,
            iter::once((
                TokenId::Nep171(withdraw.token.clone(), withdraw.token_id.clone()),
                1,
            ))
            .chain(withdraw.storage_deposit.map(|amount| {
                (
                    TokenId::Nep141(self.wnear_id().into_owned()),
                    amount.as_yoctonear(),
                )
            })),
        )
    }

    fn mt_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: MtWithdraw) -> Result<()> {
        if withdraw.token_ids.len() != withdraw.amounts.len() || withdraw.token_ids.is_empty() {
            return Err(DefuseError::InvalidIntent);
        }

        self.internal_sub_balance(
            owner_id,
            iter::repeat(withdraw.token.clone())
                .zip(withdraw.token_ids.iter().cloned())
                .map(|(token, token_id)| TokenId::Nep245(token, token_id))
                .zip(withdraw.amounts.iter().map(|a| a.0))
                .chain(withdraw.storage_deposit.map(|amount| {
                    (
                        TokenId::Nep141(self.wnear_id().into_owned()),
                        amount.as_yoctonear(),
                    )
                })),
        )
    }

    fn native_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NativeWithdraw) -> Result<()> {
        self.internal_sub_balance(
            owner_id,
            [(
                TokenId::Nep141(self.wnear_id().into_owned()),
                withdraw.amount.as_yoctonear(),
            )],
        )
    }
}
