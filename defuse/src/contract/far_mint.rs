use defuse_core::token_id::TokenId;

use defuse_near_utils::UnwrapOrPanic;

use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, assert_one_yocto, json_types::U128, near};

use crate::{
    contract::{Contract, ContractExt, Role},
    far_mint::FarMint,
};

#[near]
impl FarMint for Contract {
    #[access_control_any(roles(Role::DAO, Role::FarMintManager))]
    #[payable]
    fn mint_tokens(&mut self, receiver_id: AccountId, token_id: TokenId, amount: U128) {
        assert_one_yocto();

        self.deposit(receiver_id, [(token_id, amount.0)], Some("mint"))
            .unwrap_or_panic();
    }
}
