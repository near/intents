use defuse_core::{
    Deadline, Nonce,
    intents::{DefuseIntents, Intent},
    nep413::Nep413Payload,
    payload::{multi::MultiPayload, nep413::Nep413DefuseMessage},
};
use defuse_sandbox::{Account, SigningAccount, anyhow};
use near_sdk::{AccountIdRef, serde::Serialize, serde_json};

use crate::defuse::nonce::GenerateNonceExt;

pub trait DefuseSignerExt {
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

impl DefuseSignerExt for SigningAccount {
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

#[allow(async_fn_in_trait)]
pub trait DefaultDefuseSignerExt: DefuseSignerExt + GenerateNonceExt {
    async fn sign_defuse_payload_default<T>(
        &self,
        defuse_contract: &Account,
        intents: impl IntoIterator<Item = T>,
    ) -> anyhow::Result<MultiPayload>
    where
        T: Into<Intent>,
    {
        let deadline = Deadline::timeout(std::time::Duration::from_secs(120));
        let nonce = self
            .generate_unique_nonce(defuse_contract, Some(deadline))
            .await?;

        let defuse_intents = DefuseIntents {
            intents: intents.into_iter().map(Into::into).collect(),
        };
        Ok(self
            .sign_defuse_message(defuse_contract.id(), nonce, deadline, defuse_intents)
            .await)
    }
}
impl<T> DefaultDefuseSignerExt for T where T: DefuseSignerExt + GenerateNonceExt {}
