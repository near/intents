mod v0_4_1;

use std::{borrow::Cow, collections::BTreeMap};

use defuse_crypto::PublicKey;
use defuse_fees::Pips;
use defuse_token_id::TokenId;
use near_sdk::{AccountId, AccountIdRef, Gas, NearToken, json_types::U128, serde_json};
use rstest::rstest;

use crate::{
    Salt,
    accounts::{AccountEvent, NonceEvent, PublicKeyEvent, SaltRotationEvent},
    amounts::Amounts,
    events::{DefuseEvent, tests::v0_4_1::DefuseEventV0_4_1},
    fees::{FeeChangedEvent, FeeCollectorChangedEvent},
    intents::{
        MaybeIntentEvent,
        account::SetAuthByPredecessorId,
        token_diff::{TokenDiff, TokenDiffEvent},
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit},
    },
    tokens::TransferEvent,
};

#[cfg(feature = "imt")]
use crate::{intents::imt::ImtBurn, tokens::imt::ImtMintEvent};

// NOTE:
// 1. Adding a new event does not require backward compatibility
// 2. Modifying an existing event requires a backward compatibility test
#[derive(Debug)]
enum DefuseEventVersion {
    V0_4_1,
}

impl DefuseEventVersion {
    /// Ensures that events emitted by the current contract version
    /// can still be deserialized by older client versions
    fn assert_compatible(&self, event: &DefuseEvent) {
        let json = serde_json::to_string(event).expect("serialize new event");

        let _ = match self {
            Self::V0_4_1 => {
                match event {
                    #[cfg(feature = "imt")]
                    DefuseEvent::ImtMint(_) | DefuseEvent::ImtBurn(_) => {
                        // These events were added in v0.4.2, so they are not expected to be compatible with v0.4.1
                        return;
                    }
                    _ => serde_json::from_str::<DefuseEventV0_4_1>(&json)
                        .expect("deserialize with old event version"),
                }
            }
        };
    }
}

fn account<'a>() -> Cow<'a, AccountIdRef> {
    Cow::Owned("alice.near".parse::<AccountId>().unwrap())
}

fn pub_key<'a>() -> Cow<'a, PublicKey> {
    Cow::Owned("ed25519:11111111111111111111111111111111".parse().unwrap())
}

fn tokens() -> Amounts {
    Amounts::new(
        [
            (TokenId::Nep141("token.near".parse().unwrap()), 100),
            (TokenId::Nep245("token.near:abcd".parse().unwrap()), 200),
            (TokenId::Nep171("token.near:abcd".parse().unwrap()), 1),
        ]
        .into(),
    )
}

fn pk_added_direct_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::PublicKeyAdded(MaybeIntentEvent::new(AccountEvent {
        account_id: account(),
        event: PublicKeyEvent {
            public_key: pub_key(),
        },
    }))
}

fn pk_added_intent_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::PublicKeyAdded(MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: PublicKeyEvent {
                public_key: pub_key(),
            },
        },
        [1; 32],
    ))
}

fn fee_changed_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::FeeChanged(FeeChangedEvent {
        old_fee: Pips::from_pips(100).unwrap(),
        new_fee: Pips::from_pips(200).unwrap(),
    })
}

fn fee_collector_changed_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::FeeCollectorChanged(FeeCollectorChangedEvent {
        old_fee_collector: account(),
        new_fee_collector: account(),
    })
}

fn transfer_intent_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::Transfer(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: TransferEvent {
                receiver_id: account(),
                tokens: tokens(),
                memo: Some(Cow::Borrowed("test transfer")),
            },
        },
        [0; 32],
    )]))
}

fn token_diff_intent_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::TokenDiff(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: TokenDiffEvent {
                fees_collected: tokens(),
                diff: Cow::Owned(TokenDiff {
                    diff: Amounts::new(
                        [
                            (TokenId::Nep141("token.near".parse().unwrap()), 100),
                            (TokenId::Nep245("token.near:abcd".parse().unwrap()), -200),
                        ]
                        .into(),
                    ),
                    memo: Some("test token diff".to_string()),
                    referral: Some(account().into()),
                }),
            },
        },
        [0; 32],
    )]))
}

fn intents_executed_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::IntentsExecuted(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: NonceEvent { nonce: [0; 32] },
        },
        [0; 32],
    )]))
}

fn ft_withdraw_intent_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::FtWithdraw(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: Cow::Owned(FtWithdraw {
                token: "token.near".parse().unwrap(),
                amount: 100.into(),
                memo: Some("test ft withdraw".to_string()),
                receiver_id: account().into(),
                msg: Some("test message".to_string()),
                storage_deposit: Some(NearToken::from_yoctonear(100)),
                min_gas: Some(Gas::from_tgas(10)),
            }),
        },
        [0; 32],
    )]))
}

fn mt_withdraw_intent_event<'a>() -> DefuseEvent<'a> {
    let (token_ids, amounts): (Vec<_>, Vec<_>) = tokens()
        .into_iter()
        .map(|(token_id, amount)| (token_id.to_string(), U128::from(amount)))
        .unzip();

    DefuseEvent::MtWithdraw(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: Cow::Owned(MtWithdraw {
                token: account().into(),
                token_ids,
                amounts,
                memo: Some("test mt withdraw".to_string()),
                receiver_id: account().into(),
                msg: Some("test message".to_string()),
                storage_deposit: Some(NearToken::from_yoctonear(100)),
                min_gas: Some(Gas::from_tgas(10)),
            }),
        },
        [0; 32],
    )]))
}

fn nft_withdraw_intent_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::NftWithdraw(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: Cow::Owned(NftWithdraw {
                token: account().into(),
                token_id: "token_id1".to_string(),
                memo: Some("test nft withdraw".to_string()),
                receiver_id: account().into(),
                msg: Some("test message".to_string()),
                storage_deposit: Some(NearToken::from_yoctonear(100)),
                min_gas: Some(Gas::from_tgas(10)),
            }),
        },
        [0; 32],
    )]))
}

fn native_withdraw_intent_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::NativeWithdraw(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: Cow::Owned(NativeWithdraw {
                amount: NearToken::from_near(100),
                receiver_id: account().into(),
            }),
        },
        [0; 32],
    )]))
}

fn storage_deposit_intent_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::StorageDeposit(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: Cow::Owned(StorageDeposit {
                contract_id: account().into(),
                amount: NearToken::from_yoctonear(100),
                deposit_for_account_id: account().into(),
            }),
        },
        [0; 32],
    )]))
}

#[cfg(feature = "imt")]
fn imt_mint_intent_event<'a>() -> DefuseEvent<'a> {
    let tokens = Amounts::new(
        tokens()
            .into_iter()
            .map(|(token_id, amount)| (token_id.to_string(), amount))
            .collect::<BTreeMap<_, _>>(),
    );

    DefuseEvent::ImtMint(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: ImtMintEvent {
                receiver_id: account(),
                tokens,
                memo: Some(Cow::Borrowed("test imt mint")),
            },
        },
        [0; 32],
    )]))
}

#[cfg(feature = "imt")]
fn imt_burn_intent_event<'a>() -> DefuseEvent<'a> {
    let tokens = Amounts::new(
        tokens()
            .into_iter()
            .map(|(token_id, amount)| (token_id.to_string(), amount))
            .collect::<BTreeMap<_, _>>(),
    );

    DefuseEvent::ImtBurn(Cow::Owned(vec![MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: Cow::Owned(ImtBurn {
                minter_id: account().into(),
                tokens,
                memo: Some("test imt burn".to_string()),
            }),
        },
        [0; 32],
    )]))
}

fn account_locked_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::AccountLocked(AccountEvent {
        account_id: account(),
        event: (),
    })
}

fn account_unlocked_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::AccountUnlocked(AccountEvent {
        account_id: account(),
        event: (),
    })
}

fn set_auth_by_predecessor_id_intent_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::SetAuthByPredecessorId(MaybeIntentEvent::new_with_hash(
        AccountEvent {
            account_id: account(),
            event: Cow::Owned(SetAuthByPredecessorId { enabled: true }),
        },
        [0; 32],
    ))
}

fn set_auth_by_predecessor_id_direct_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::SetAuthByPredecessorId(MaybeIntentEvent::new(AccountEvent {
        account_id: account(),
        event: Cow::Owned(SetAuthByPredecessorId { enabled: true }),
    }))
}

fn salt_rotation_event<'a>() -> DefuseEvent<'a> {
    DefuseEvent::SaltRotation(SaltRotationEvent {
        current: Salt::derive(3),
        invalidated: [Salt::derive(2), Salt::derive(1)].into_iter().collect(),
    })
}

fn get_all_events<'a>() -> Vec<DefuseEvent<'a>> {
    let mut all_events = vec![
        pk_added_direct_event(),
        pk_added_intent_event(),
        fee_changed_event(),
        fee_collector_changed_event(),
        transfer_intent_event(),
        token_diff_intent_event(),
        intents_executed_event(),
        ft_withdraw_intent_event(),
        mt_withdraw_intent_event(),
        nft_withdraw_intent_event(),
        native_withdraw_intent_event(),
        storage_deposit_intent_event(),
        account_locked_event(),
        account_unlocked_event(),
        set_auth_by_predecessor_id_intent_event(),
        set_auth_by_predecessor_id_direct_event(),
        salt_rotation_event(),
    ];

    #[cfg(feature = "imt")]
    {
        all_events.extend([imt_mint_intent_event(), imt_burn_intent_event()]);
    }

    all_events
}

#[rstest]
#[case(DefuseEventVersion::V0_4_1)]
fn event_backward_compatibility_test(#[case] event_version: DefuseEventVersion) {
    get_all_events()
        .iter()
        .for_each(|event| event_version.assert_compatible(event));
}
