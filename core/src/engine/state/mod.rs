pub mod cached;
pub mod deltas;

use std::borrow::Cow;

use cached::CachedState;
use defuse_crypto::PublicKey;
use impl_tools::autoimpl;
use near_sdk::{AccountId, AccountIdRef};

use crate::{
    Nonce, Result,
    fees::Pips,
    intents::tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw},
    tokens::TokenId,
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

    fn ft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: FtWithdraw) -> Result<()>;

    fn nft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NftWithdraw) -> Result<()>;

    fn mt_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: MtWithdraw) -> Result<()>;

    fn native_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NativeWithdraw) -> Result<()>;
}
