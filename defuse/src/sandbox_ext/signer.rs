use defuse_core::{
    Deadline, Nonce,
    nep413::Nep413Payload,
    payload::{multi::MultiPayload, nep413::Nep413DefuseMessage},
};
use defuse_sandbox::SigningAccount;
use near_sdk::{AccountIdRef, serde::Serialize, serde_json};

pub trait DefuseSigner {
    async fn sign_defuse_message<T>(
        &self,
        defuse_contract: impl AsRef<AccountIdRef>,
        nonce: Nonce,
        deadline: Deadline,
        message: T,
    ) -> MultiPayload
    where
        T: Serialize;
}

impl DefuseSigner for SigningAccount {
    async fn sign_defuse_message<T>(
        &self,
        defuse_contract: impl AsRef<AccountIdRef>,
        nonce: Nonce,
        deadline: Deadline,
        message: T,
    ) -> MultiPayload
    where
        T: Serialize,
    {
        self.sign_nep413(
            Nep413Payload::new(
                serde_json::to_string(&Nep413DefuseMessage {
                    signer_id: self.id().clone(),
                    deadline,
                    message,
                })
                .unwrap(),
            )
            .with_recipient(defuse_contract.as_ref())
            .with_nonce(nonce),
        )
        .await
        .unwrap()
        .into()
    }
}
