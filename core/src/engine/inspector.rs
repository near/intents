use crate::{
    Deadline,
    intents::{
        token_diff::TokenDiff,
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit, Transfer},
    },
    tokens::Amounts,
};
use impl_tools::autoimpl;
use near_sdk::{AccountIdRef, CryptoHash};

#[autoimpl(for <T: trait + ?Sized> &mut T, Box<T>)]
pub trait Inspector {
    fn on_deadline(&mut self, deadline: Deadline);

    fn on_transfer(
        &mut self,
        sender_id: &AccountIdRef,
        transfer: &Transfer,
        intent_hash: CryptoHash,
    );
    fn on_token_diff(
        &mut self,
        owner_id: &AccountIdRef,
        token_diff: &TokenDiff,
        fees_collected: &Amounts,
        intent_hash: CryptoHash,
    );

    fn on_ft_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        ft_withdraw: FtWithdraw,
        intent_hash: CryptoHash,
    );

    fn on_nft_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        nft_withdraw: NftWithdraw,
        intent_hash: CryptoHash,
    );

    fn on_mt_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        mt_withdraw: MtWithdraw,
        intent_hash: CryptoHash,
    );

    fn on_native_withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        native_withdraw: NativeWithdraw,
        intent_hash: CryptoHash,
    );

    fn on_storage_deposit(
        &mut self,
        owner_id: &AccountIdRef,
        storage_deposit: StorageDeposit,
        intent_hash: CryptoHash,
    );

    fn on_intent_executed(&mut self, signer_id: &AccountIdRef, hash: CryptoHash);
}
