// use std::time::Duration;

// use defuse_core::fees::Pips;
// use defuse_core::intents::{DefuseIntents, Intent};
// use defuse_core::payload::multi::MultiPayload;
// use defuse_core::{Deadline, ExpirableNonce, Nonce, SaltedNonce, VersionedNonce};
// use defuse_core::{
//     crypto::Payload,
//     nep413::{Nep413Payload, SignedNep413Payload},
//     sep53::{Sep53Payload, SignedSep53Payload},
//     ton_connect::{SignedTonConnectPayload, TonConnectPayload},
// };
// use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
// use near_sdk::serde::Serialize;
// use near_sdk::{AccountId, NearToken, serde_json::json};

// use crate::extensions::state::SaltManagerViewExt;

// pub trait Signer {
//     // fn secret_key(&self) -> ::SecretKey;

//     // fn sign_nep413(&self, payload: Nep413Payload) -> SignedNep413Payload;
//     // fn sign_ton_connect(&self, payload: TonConnectPayload) -> SignedTonConnectPayload;
//     // fn sign_sep53(&self, payload: Sep53Payload) -> SignedSep53Payload;
// }

// impl Signer for SigningAccount {
//     // fn secret_key(&self) -> near_crypto::SecretKey {
//     //     // near_sdk does not expose near_crypto API
//     //     self.secret_key().to_string().parse().unwrap()
//     // }

//     // fn sign_nep413(&self, payload: Nep413Payload) -> SignedNep413Payload {
//     //     let secret_key = Signer::secret_key(self);

//     //     match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
//     //         (near_crypto::Signature::ED25519(sig), near_crypto::PublicKey::ED25519(pk)) => {
//     //             SignedNep413Payload {
//     //                 payload,
//     //                 public_key: pk.0,
//     //                 signature: sig.to_bytes(),
//     //             }
//     //         }
//     //         _ => unreachable!(),
//     //     }
//     // }

//     // fn sign_ton_connect(&self, payload: TonConnectPayload) -> SignedTonConnectPayload {
//     //     let secret_key = Signer::secret_key(self);

//     //     match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
//     //         (near_crypto::Signature::ED25519(sig), near_crypto::PublicKey::ED25519(pk)) => {
//     //             SignedTonConnectPayload {
//     //                 payload,
//     //                 public_key: pk.0,
//     //                 signature: sig.to_bytes(),
//     //             }
//     //         }
//     //         _ => unreachable!(),
//     //     }
//     // }

//     // fn sign_sep53(&self, payload: Sep53Payload) -> SignedSep53Payload {
//     //     let secret_key = Signer::secret_key(self);

//     //     match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
//     //         (near_crypto::Signature::ED25519(sig), near_crypto::PublicKey::ED25519(pk)) => {
//     //             SignedSep53Payload {
//     //                 payload,
//     //                 public_key: pk.0,
//     //                 signature: sig.to_bytes(),
//     //             }
//     //         }
//     //         _ => unreachable!(),
//     //     }
//     // }
// }

// pub trait DefuseSigner: Signer {
//     #[must_use]
//     fn sign_defuse_message<T>(
//         &self,
//         signing_standard: SigningStandard,
//         defuse_contract: &AccountId,
//         nonce: Nonce,
//         deadline: Deadline,
//         message: T,
//     ) -> MultiPayload
//     where
//         T: Serialize;
// }

// pub trait DefuseSignerExt: DefuseSigner + SaltManagerViewExt {
//     async fn unique_nonce(
//         &self,
//         defuse_contract_id: &AccountId,
//         deadline: Option<Deadline>,
//     ) -> anyhow::Result<Nonce> {
//         let deadline = deadline.unwrap_or_else(|| Deadline::timeout(Duration::from_secs(120)));

//         let salt = self
//             .current_salt(defuse_contract_id)
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
// impl<T> DefuseSignerExt for T where T: DefuseSigner + SaltManagerViewExt {}

// impl DefuseSigner for Account {
//     fn sign_defuse_message<T>(
//         &self,
//         signing_standard: SigningStandard,
//         defuse_contract: &AccountId,
//         nonce: Nonce,
//         deadline: Deadline,
//         message: T,
//     ) -> MultiPayload
//     where
//         T: Serialize,
//     {
//         match signing_standard {
//             SigningStandard::Nep413 => self
//                 .sign_nep413(
//                     Nep413Payload::new(
//                         serde_json::to_string(&Nep413DefuseMessage {
//                             signer_id: self.id().clone(),
//                             deadline,
//                             message,
//                         })
//                         .unwrap(),
//                     )
//                     .with_recipient(defuse_contract)
//                     .with_nonce(nonce),
//                 )
//                 .into(),
//             SigningStandard::TonConnect => self
//                 .sign_ton_connect(TonConnectPayload {
//                     address: MsgAddress::arbitrary(&mut Unstructured::new(
//                         self.secret_key().public_key().key_data(),
//                     ))
//                     .unwrap(),
//                     domain: "intents.test.near".to_string(),
//                     timestamp: defuse_near_utils::time::now(),
//                     payload: defuse::core::ton_connect::TonConnectPayloadSchema::text(
//                         serde_json::to_string(&DefusePayload {
//                             signer_id: self.id().clone(),
//                             verifying_contract: defuse_contract.clone(),
//                             deadline,
//                             nonce,
//                             message,
//                         })
//                         .unwrap(),
//                     ),
//                 })
//                 .into(),
//             SigningStandard::Sep53 => self
//                 .sign_sep53(Sep53Payload::new(
//                     serde_json::to_string(&DefusePayload {
//                         signer_id: self.id().clone(),
//                         verifying_contract: defuse_contract.clone(),
//                         deadline,
//                         nonce,
//                         message,
//                     })
//                     .unwrap(),
//                 ))
//                 .into(),
//         }
//     }
// }

// // #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
// #[derive(Debug, Default)]
// pub enum SigningStandard {
//     #[default]
//     Nep413,
//     Erc191,
//     Tip191,
//     RawEd25519,
//     WebAuthn,
//     TonConnect,
//     Sep53,
// }
