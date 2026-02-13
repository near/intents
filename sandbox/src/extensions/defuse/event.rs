use defuse::core::accounts::{AccountEvent, NonceEvent, PublicKeyEvent};
use defuse::core::amounts::Amounts;
use defuse::core::events::DefuseEvent;
use defuse::core::intents::account::{AddPublicKey, RemovePublicKey, SetAuthByPredecessorId};
use defuse::core::intents::token_diff::{TokenDiff, TokenDiffEvent};
use defuse::core::intents::tokens::{
    FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit, Transfer,
};
use defuse::core::intents::{Intent, MaybeIntentEvent};
use defuse::core::payload::{DefusePayload, ExtractDefusePayload};
use defuse::core::tokens::TransferEvent;
use defuse::core::{intents::DefuseIntents, payload::multi::MultiPayload};
use defuse_crypto::Payload;
use near_sdk::{AccountId, AsNep297Event, CryptoHash};
use std::borrow::Cow;

#[cfg(feature = "imt")]
use defuse::core::{
    intents::tokens::imt::{ImtBurn, ImtMint},
    tokens::imt::ImtMintEvent,
};

pub trait ToEventLog {
    fn to_event_log(self) -> Vec<String>;
}

impl ToEventLog for MultiPayload {
    #[inline]
    fn to_event_log(self) -> Vec<String> {
        [self].to_event_log()
    }
}

// TODO: add emitted mt event logs
impl<const N: usize> ToEventLog for [MultiPayload; N] {
    #[inline]
    fn to_event_log(self) -> Vec<String> {
        self.to_defuse_events()
            .into_iter()
            .map(|e| e.to_nep297_event().to_event_log())
            .collect()
    }
}

trait PayloadToDefuseEvents {
    fn to_defuse_events(self) -> Vec<DefuseEvent<'static>>;
}

impl<I> PayloadToDefuseEvents for I
where
    I: IntoIterator<Item = MultiPayload>,
{
    fn to_defuse_events(self) -> Vec<DefuseEvent<'static>> {
        let (nonce_events, intent_events): (Vec<_>, Vec<Vec<_>>) = self
            .into_iter()
            .map(|payload| {
                let hash = payload.hash();

                let DefusePayload::<DefuseIntents> {
                    signer_id,
                    nonce,
                    message,
                    ..
                } = payload
                    .extract_defuse_payload()
                    .unwrap_or_else(|_| unreachable!("invalid payload in tests"));

                let nonce_event = MaybeIntentEvent::new_with_hash(
                    AccountEvent::new(signer_id.clone(), NonceEvent::new(nonce)),
                    hash,
                );

                let intent_events = message
                    .intents
                    .into_iter()
                    .flat_map(|i| i.into_defuse_events(signer_id.clone(), hash))
                    .collect();

                (nonce_event, intent_events)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .unzip();

        let final_event =
            (!nonce_events.is_empty()).then(|| DefuseEvent::IntentsExecuted(nonce_events.into()));

        intent_events
            .into_iter()
            .flatten()
            .chain(final_event)
            .collect()
    }
}

trait IntoDefuseEvents<'a> {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>>;
}

impl<'a> IntoDefuseEvents<'a> for Intent {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        match self {
            Self::AddPublicKey(intent) => intent.into_defuse_events(signer_id, intent_hash),
            Self::RemovePublicKey(intent) => intent.into_defuse_events(signer_id, intent_hash),
            Self::SetAuthByPredecessorId(intent) => {
                intent.into_defuse_events(signer_id, intent_hash)
            }
            Self::Transfer(intent) => intent.into_defuse_events(signer_id, intent_hash),
            Self::FtWithdraw(intent) => intent.into_defuse_events(signer_id, intent_hash),
            Self::NftWithdraw(intent) => intent.into_defuse_events(signer_id, intent_hash),
            Self::MtWithdraw(intent) => intent.into_defuse_events(signer_id, intent_hash),
            Self::NativeWithdraw(intent) => intent.into_defuse_events(signer_id, intent_hash),
            Self::StorageDeposit(intent) => intent.into_defuse_events(signer_id, intent_hash),
            Self::TokenDiff(intent) => intent.into_defuse_events(signer_id, intent_hash),
            Self::AuthCall(_) => vec![],
            #[cfg(feature = "imt")]
            Self::ImtMint(intent) => intent.into_defuse_events(signer_id, intent_hash),
            #[cfg(feature = "imt")]
            Self::ImtBurn(intent) => intent.into_defuse_events(signer_id, intent_hash),
        }
    }
}

impl<'a> IntoDefuseEvents<'a> for SetAuthByPredecessorId {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::SetAuthByPredecessorId(
            MaybeIntentEvent::new_with_hash(
                AccountEvent::new(Cow::Owned(signer_id), Cow::Owned(self)),
                intent_hash,
            ),
        )]
    }
}

impl<'a> IntoDefuseEvents<'a> for RemovePublicKey {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::PublicKeyRemoved(
            MaybeIntentEvent::new_with_hash(
                AccountEvent::new(
                    Cow::Owned(signer_id),
                    PublicKeyEvent {
                        public_key: Cow::Owned(self.public_key),
                    },
                ),
                intent_hash,
            ),
        )]
    }
}

impl<'a> IntoDefuseEvents<'a> for AddPublicKey {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::PublicKeyAdded(
            MaybeIntentEvent::new_with_hash(
                AccountEvent::new(
                    Cow::Owned(signer_id),
                    PublicKeyEvent {
                        public_key: Cow::Owned(self.public_key),
                    },
                ),
                intent_hash,
            ),
        )]
    }
}

impl<'a> IntoDefuseEvents<'a> for Transfer {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::Transfer(Cow::Owned(
            [MaybeIntentEvent::new_with_hash(
                AccountEvent::new(
                    signer_id,
                    TransferEvent {
                        receiver_id: Cow::Owned(self.receiver_id),
                        tokens: self.tokens,
                        memo: self.memo.map(Cow::Owned),
                    },
                ),
                intent_hash,
            )]
            .to_vec(),
        ))]
    }
}

impl<'a> IntoDefuseEvents<'a> for FtWithdraw {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::FtWithdraw(Cow::Owned(
            [MaybeIntentEvent::new_with_hash(
                AccountEvent::new(signer_id, Cow::Owned(self)),
                intent_hash,
            )]
            .to_vec(),
        ))]
    }
}

impl<'a> IntoDefuseEvents<'a> for NftWithdraw {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::NftWithdraw(Cow::Owned(
            [MaybeIntentEvent::new_with_hash(
                AccountEvent::new(signer_id, Cow::Owned(self)),
                intent_hash,
            )]
            .to_vec(),
        ))]
    }
}

impl<'a> IntoDefuseEvents<'a> for MtWithdraw {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::MtWithdraw(Cow::Owned(
            [MaybeIntentEvent::new_with_hash(
                AccountEvent::new(signer_id, Cow::Owned(self)),
                intent_hash,
            )]
            .to_vec(),
        ))]
    }
}

impl<'a> IntoDefuseEvents<'a> for StorageDeposit {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::StorageDeposit(Cow::Owned(
            [MaybeIntentEvent::new_with_hash(
                AccountEvent::new(signer_id, Cow::Owned(self)),
                intent_hash,
            )]
            .to_vec(),
        ))]
    }
}

impl<'a> IntoDefuseEvents<'a> for NativeWithdraw {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::NativeWithdraw(Cow::Owned(
            [MaybeIntentEvent::new_with_hash(
                AccountEvent::new(signer_id, Cow::Owned(self)),
                intent_hash,
            )]
            .to_vec(),
        ))]
    }
}

impl<'a> IntoDefuseEvents<'a> for TokenDiff {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::TokenDiff(Cow::Owned(
            [MaybeIntentEvent::new_with_hash(
                AccountEvent::new(
                    signer_id,
                    TokenDiffEvent {
                        diff: Cow::Owned(self),
                        // 0 fee in tests
                        fees_collected: Amounts::default(),
                    },
                ),
                intent_hash,
            )]
            .to_vec(),
        ))]
    }
}

#[cfg(feature = "imt")]
impl<'a> IntoDefuseEvents<'a> for ImtMint {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::ImtMint(Cow::Owned(
            [MaybeIntentEvent::new_with_hash(
                AccountEvent::new(
                    signer_id,
                    ImtMintEvent {
                        receiver_id: Cow::Owned(self.receiver_id),
                        tokens: self.tokens.clone(),
                        memo: self.memo.map(Cow::Owned),
                    },
                ),
                intent_hash,
            )]
            .to_vec(),
        ))]
    }
}

#[cfg(feature = "imt")]
impl<'a> IntoDefuseEvents<'a> for ImtBurn {
    fn into_defuse_events(
        self,
        signer_id: AccountId,
        intent_hash: CryptoHash,
    ) -> Vec<DefuseEvent<'a>> {
        vec![DefuseEvent::ImtBurn(Cow::Owned(vec![
            MaybeIntentEvent::new_with_hash(
                AccountEvent {
                    account_id: signer_id.into(),
                    event: Cow::Owned(self),
                },
                intent_hash,
            ),
        ]))]
    }
}
