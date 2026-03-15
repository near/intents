use defuse_wallet::signature::RequestMessage;
use near_kit::{CryptoHash, Near, NearToken};
use near_sdk::{
    GlobalContractId,
    serde_json::json,
    state_init::{StateInit, StateInitV1},
};

pub struct Relayer {
    client: Near,
}

pub struct RelayRequest {
    pub state_init: Option<StateInit>,
    pub msg: RequestMessage,
    pub proof: String,
}

impl Relayer {
    pub async fn relay(&self, request: RelayRequest) {
        let mut tx = self.client.transaction(request.msg.signer_id);

        if let Some(state_init) = request.state_init {
            if state_init.derive_account_id() != request.msg.signer_id {
                todo!("wrong state_init");
            }

            let StateInit::V1(state_init) = state_init else {
                unimplemented!();
            };

            tx = match state_init.code {
                GlobalContractId::CodeHash(hash) => tx.state_init_by_hash(
                    CryptoHash::from_bytes(hash.into()),
                    state_init.data,
                    NearToken::ZERO,
                ),
                GlobalContractId::AccountId(account_id) => {
                    tx.state_init_by_publisher(account_id, state_init.data, NearToken::ZERO)
                }
            };
        }
        tx = tx
            .call("w_execute_signed")
            .args(json!({
                "msg": request.msg,
                "proof": request.proof,
            }))
            .finish();
    }
}
