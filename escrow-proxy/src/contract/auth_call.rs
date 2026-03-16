use defuse_auth_call::AuthCallee;
use defuse_escrow_swap::ext_escrow;
use near_sdk::{AccountId, Gas, PromiseOrValue, env, near, require, serde_json};

use super::{Contract, ContractExt};

use defuse_escrow_swap::Params as EscrowParams;

#[near(serializers = [json])]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum OnAuthMessage {
    CancelEscrow {
        escrow_address: AccountId,
        params: EscrowParams,
    },
}

#[near]
impl AuthCallee for Contract {
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        require!(
            env::predecessor_account_id() == self.0.config().on_auth_caller,
            "unauthorized on_auth_caller"
        );

        require!(signer_id == self.0.config().owner_id, "unauthorized signer");

        let msg: OnAuthMessage = serde_json::from_str(&msg).unwrap();
        match msg {
            OnAuthMessage::CancelEscrow {
                escrow_address,
                params,
            } => PromiseOrValue::Promise(
                ext_escrow::ext(escrow_address)
                    .with_static_gas(Gas::from_tgas(50))
                    .with_unused_gas_weight(1)
                    .es_close(params),
            ),
        }
    }
}
