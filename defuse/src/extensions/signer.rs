use arbitrary::{Arbitrary, Unstructured};

use defuse_core::{
    Deadline, Nonce,
    crypto::Payload,
    nep413::{Nep413Payload, SignedNep413Payload},
    payload::{DefusePayload, multi::MultiPayload, nep413::Nep413DefuseMessage},
    sep53::{Sep53Payload, SignedSep53Payload},
    ton_connect::{SignedTonConnectPayload, TonConnectPayload, tlb_ton::MsgAddress},
};
use defuse_sandbox::{
    SigningAccount,
    api::{PublicKey, SecretKey, types::Signature},
};
use near_sdk::{AccountId, serde::Serialize, serde_json};

pub trait Signer {
    fn secret_key(&self) -> &SecretKey;

    fn sign_nep413(&self, payload: Nep413Payload) -> SignedNep413Payload;
    fn sign_ton_connect(&self, payload: TonConnectPayload) -> SignedTonConnectPayload;
    fn sign_sep53(&self, payload: Sep53Payload) -> SignedSep53Payload;
}

impl Signer for SigningAccount {
    fn secret_key(&self) -> &SecretKey {
        self.private_key()
    }

    fn sign_nep413(&self, payload: Nep413Payload) -> SignedNep413Payload {
        let secret_key = Signer::secret_key(self);

        match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
            (Signature::ED25519(sig), PublicKey::ED25519(pk)) => SignedNep413Payload {
                payload,
                public_key: pk.0,
                signature: sig.to_bytes(),
            },
            _ => unreachable!(),
        }
    }

    fn sign_ton_connect(&self, payload: TonConnectPayload) -> SignedTonConnectPayload {
        let secret_key = Signer::secret_key(self);

        match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
            (Signature::ED25519(sig), PublicKey::ED25519(pk)) => SignedTonConnectPayload {
                payload,
                public_key: pk.0,
                signature: sig.to_bytes(),
            },
            _ => unreachable!(),
        }
    }

    fn sign_sep53(&self, payload: Sep53Payload) -> SignedSep53Payload {
        let secret_key = Signer::secret_key(self);

        match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
            (Signature::ED25519(sig), PublicKey::ED25519(pk)) => SignedSep53Payload {
                payload,
                public_key: pk.0,
                signature: sig.to_bytes(),
            },
            _ => unreachable!(),
        }
    }
}

pub trait DefuseSigner: Signer {
    #[must_use]
    fn sign_defuse_message<T>(
        &self,
        signing_standard: SigningStandard,
        defuse_contract: &AccountId,
        nonce: Nonce,
        deadline: Deadline,
        message: T,
    ) -> MultiPayload
    where
        T: Serialize;
}

impl DefuseSigner for SigningAccount {
    fn sign_defuse_message<T>(
        &self,
        signing_standard: SigningStandard,
        defuse_contract: &AccountId,
        nonce: Nonce,
        deadline: Deadline,
        message: T,
    ) -> MultiPayload
    where
        T: Serialize,
    {
        match signing_standard {
            SigningStandard::Nep413 => self
                .sign_nep413(
                    Nep413Payload::new(
                        serde_json::to_string(&Nep413DefuseMessage {
                            signer_id: self.id().clone(),
                            deadline,
                            message,
                        })
                        .unwrap(),
                    )
                    .with_recipient(defuse_contract)
                    .with_nonce(nonce),
                )
                .into(),
            SigningStandard::TonConnect => self
                .sign_ton_connect(TonConnectPayload {
                    address: MsgAddress::arbitrary(&mut Unstructured::new(
                        self.secret_key().public_key().key_data(),
                    ))
                    .unwrap(),
                    domain: "intents.test.near".to_string(),
                    timestamp: defuse_near_utils::time::now(),
                    payload: defuse_core::ton_connect::TonConnectPayloadSchema::text(
                        serde_json::to_string(&DefusePayload {
                            signer_id: self.id().clone(),
                            verifying_contract: defuse_contract.clone(),
                            deadline,
                            nonce,
                            message,
                        })
                        .unwrap(),
                    ),
                })
                .into(),
            SigningStandard::Sep53 => self
                .sign_sep53(Sep53Payload::new(
                    serde_json::to_string(&DefusePayload {
                        signer_id: self.id().clone(),
                        verifying_contract: defuse_contract.clone(),
                        deadline,
                        nonce,
                        message,
                    })
                    .unwrap(),
                ))
                .into(),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Default, Arbitrary)]
pub enum SigningStandard {
    #[default]
    Nep413,
    TonConnect,
    Sep53,
    // Erc191,
    // Tip191,
    // RawEd25519,
    // WebAuthn,
}

// impl<T> DefuseSignerExt for T where T: DefuseSigner + SaltManagerViewExt {}

// pub trait DefuseSignerExt: DefuseSigner + SaltManagerViewExt {
//     async fn unique_nonce(
//         &self,
//         defuse_contract_id: &AccountId,
//         deadline: Option<Deadline>,
//     ) -> anyhow::Result<Nonce> {
//         let deadline = deadline.unwrap_or_else(|| Deadline::timeout(Duration::from_secs(120)));

//         let salt = self
//             .current_salt()
//             .await
//             .expect("should be able to fetch salt");

//         let mut nonce_bytes = [0u8; 15];
//         TestRng::from_entropy().fill_bytes(&mut nonce_bytes);

//         let salted = SaltedNonce::new(salt, ExpirableNonce::new(deadline, nonce_bytes));
//         Ok(VersionedNonce::V1(salted).into())
//     }

//     async fn sign_defuse_payload_default<T>(
//         &self,
//         defuse_contract_id: &AccountId,
//         intents: impl IntoIterator<Item = T>, //Intent>,
//     ) -> anyhow::Result<MultiPayload>
//     where
//         T: Into<Intent>,
//     {
//         let deadline = Deadline::timeout(std::time::Duration::from_secs(120));
//         let nonce = self
//             .unique_nonce(defuse_contract_id, Some(deadline))
//             .await?;

//         let defuse_intents = DefuseIntents {
//             intents: intents.into_iter().map(Into::into).collect(),
//         };

//         Ok(self.sign_defuse_message(
//             SigningStandard::default(),
//             defuse_contract_id,
//             nonce,
//             deadline,
//             defuse_intents,
//         ))
//     }
// }
