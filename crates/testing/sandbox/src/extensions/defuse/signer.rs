use crate::{anyhow, extensions::defuse::DefuseClient};
use defuse::core::{
    Deadline, Nonce,
    intents::{DefuseIntents, Intent},
    nep413::Nep413Payload,
    payload::{multi::MultiPayload, nep413::Nep413DefuseMessage},
};
use defuse_nep413::SignedNep413Payload;
use near_kit::{AccountIdRef, Near};
use serde::Serialize;
use serde_json;

use crate::extensions::defuse::nonce::generate_unique_nonce;

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

    async fn sign_defuse_payload_default<T>(
        &self,
        defuse_contract: &DefuseClient,
        intents: impl IntoIterator<Item = T>,
    ) -> anyhow::Result<MultiPayload>
    where
        T: Into<Intent>,
    {
        let deadline = Deadline::timeout(std::time::Duration::from_mins(2));
        let nonce = generate_unique_nonce(defuse_contract, Some(deadline)).await?;

        let defuse_intents = DefuseIntents {
            intents: intents.into_iter().map(Into::into).collect(),
        };
        Ok(self
            .sign_defuse_message(
                defuse_contract.contract_id(),
                nonce,
                deadline,
                defuse_intents,
            )
            .await)
    }
}

impl DefuseSignerExt for Near {
    // TODO: may be make it as part of defuse_nep413 crate?
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
        let payload = Nep413Payload::new(
            serde_json::to_string(&Nep413DefuseMessage {
                signer_id: self.account_id().clone(),
                deadline,
                message,
            })
            .unwrap(),
        )
        .with_recipient(defuse_contract.as_ref())
        .with_nonce(nonce);

        let signed = self.sign_message(payload.clone().into()).await.unwrap();

        MultiPayload::Nep413(SignedNep413Payload {
            payload,
            public_key: *signed.public_key.as_ed25519_bytes().unwrap(),
            signature: signed.signature.as_bytes().try_into().unwrap(),
        })
    }
}
