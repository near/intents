#[cfg(feature = "imt")]
pub mod imt;
pub mod nep141;
pub mod nep171;
pub mod nep245;

use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use defuse_core::payload::multi::MultiPayload;
use defuse_near_utils::REFUND_MEMO;
use near_sdk::{AccountId, account_id::ParseAccountError};
use serde::{Deserialize, Serialize};

/// Memo of a deposit's `mt_mint` event when the deposit message sets none.
pub const DEPOSIT_MEMO: &str = "deposit";

/// Maximum length of a deposit message's memo, in bytes.
pub const MAX_DEPOSIT_MEMO_LEN: usize = 256;

/// Memos the Verifier writes into its own `mt_mint` and `mt_burn` events for
/// refunds and withdrawals. Indexers rely on them, so a deposit message can't
/// set them. [`DEPOSIT_MEMO`] is allowed: it means the same as no memo.
pub const RESERVED_DEPOSIT_MEMOS: &[&str] = &[REFUND_MEMO, "withdraw"];

#[must_use]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositMessage {
    pub receiver_id: AccountId,

    /// Memo for the deposit's `mt_mint` event; [`DEPOSIT_MEMO`] when absent
    /// or empty. Must be at most [`MAX_DEPOSIT_MEMO_LEN`] bytes and not one of
    /// [`RESERVED_DEPOSIT_MEMOS`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub action: Option<DepositAction>,
}

impl DepositMessage {
    #[inline]
    pub const fn new(receiver_id: AccountId) -> Self {
        Self {
            receiver_id,
            memo: None,
            action: None,
        }
    }

    #[inline]
    pub fn with_action(mut self, action: impl Into<Option<DepositAction>>) -> Self {
        self.action = action.into();
        self
    }

    #[inline]
    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    /// Whether the message is just a receiver: no memo and nothing to execute.
    /// Such a message is written as the bare `receiver_id`.
    const fn is_plain(&self) -> bool {
        self.memo.is_none()
            && match &self.action {
                None => true,
                Some(DepositAction::Execute(exec)) => exec.execute_intents.is_empty(),
            }
    }
}

/// Memo for a deposit's `mt_mint` event: `memo` if it is valid,
/// [`DEPOSIT_MEMO`] if it is absent or empty.
#[inline]
pub fn deposit_mint_memo(memo: Option<&str>) -> Result<&str, InvalidDepositMemo> {
    let Some(memo) = memo.filter(|memo| !memo.is_empty()) else {
        return Ok(DEPOSIT_MEMO);
    };
    if memo.len() > MAX_DEPOSIT_MEMO_LEN {
        return Err(InvalidDepositMemo::TooLong(memo.len()));
    }
    if RESERVED_DEPOSIT_MEMOS.contains(&memo) {
        return Err(InvalidDepositMemo::Reserved(memo.to_owned()));
    }
    Ok(memo)
}

impl Display for DepositMessage {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_plain() {
            f.write_str(self.receiver_id.as_str())
        } else {
            f.write_str(&serde_json::to_string(self).unwrap_or_else(|e| panic!("{e}")))
        }
    }
}

impl FromStr for DepositMessage {
    type Err = ParseDepositMessageError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('{') {
            let msg: Self = serde_json::from_str(s)?;
            deposit_mint_memo(msg.memo.as_deref())?;
            Ok(msg)
        } else {
            s.parse().map(Self::new).map_err(Into::into)
        }
    }
}

#[must_use]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DepositAction {
    Execute(ExecuteIntents),
    // Notify(NotifyOnTransfer),
}

#[must_use]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteIntents {
    pub execute_intents: Vec<MultiPayload>,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub refund_if_fails: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseDepositMessageError {
    #[error(transparent)]
    Account(#[from] ParseAccountError),
    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),
    #[error(transparent)]
    Memo(#[from] InvalidDepositMemo),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidDepositMemo {
    #[error("deposit memo is too long: {0} bytes, max {MAX_DEPOSIT_MEMO_LEN}")]
    TooLong(usize),
    #[error("deposit memo `{0}` is reserved")]
    Reserved(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_simple() {
        // Simple format: just receiver_id
        let json = r#"{"receiver_id": "alice.near"}"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert!(msg.action.is_none());
    }

    // #[test]
    // fn test_deserialize_with_notify() {
    //     // With notify action (flattened untagged)
    //     let json = r#"{
    //         "receiver_id": "alice.near",
    //         "msg": "hello world",
    //         "min_gas": null
    //     }"#;
    //     let msg: DepositMessage = serde_json::from_str(json).unwrap();

    //     assert_eq!(msg.receiver_id.as_str(), "alice.near");
    //     match msg.action {
    //         Some(DepositAction::Notify(notify)) => {
    //             assert_eq!(notify.msg, "hello world");
    //             assert!(notify.min_gas.is_none());
    //         }
    //         _ => panic!("Expected Notify action"),
    //     }
    // }

    #[test]
    fn test_deserialize_with_execute() {
        // With execute action (flattened untagged)
        let json = r#"{
            "receiver_id": "alice.near",
            "execute_intents": [],
            "refund_if_fails": true
        }"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        match msg.action {
            Some(DepositAction::Execute(exec)) => {
                assert!(exec.execute_intents.is_empty());
                assert!(exec.refund_if_fails);
            }
            _ => panic!("Expected Execute action"),
        }
    }

    #[test]
    fn test_deserialize_with_memo() {
        // With memo
        let json = r#"{
            "receiver_id": "alice.near",
            "memo": "the-awesome-memo-there"
        }"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert_eq!(msg.memo.unwrap(), "the-awesome-memo-there");
    }

    #[test]
    fn test_serialize_simple() {
        // Simple message serialization
        let msg = DepositMessage::new("alice.near".parse().unwrap());
        let json = serde_json::to_string(&msg).unwrap();

        // Should serialize with just receiver_id (action omitted when None)
        assert!(json.contains("\"receiver_id\":\"alice.near\""));
        assert!(!json.contains("action"));
    }

    // #[test]
    // fn test_serialize_with_notify() {
    //     // Serialization with notify action
    //     let msg = DepositMessage {
    //         receiver_id: "alice.near".parse().unwrap(),
    //         action: Some(DepositAction::Notify(NotifyOnTransfer::new(
    //             "hello".to_string(),
    //         ))),
    //     };
    //     let json = serde_json::to_string(&msg).unwrap();

    //     // Should serialize with flattened notify fields
    //     assert!(json.contains("\"receiver_id\":\"alice.near\""));
    //     assert!(json.contains("\"msg\":\"hello\""));
    // }

    #[test]
    fn test_serialize_with_execute() {
        // Serialization with execute action
        let msg = DepositMessage {
            receiver_id: "alice.near".parse().unwrap(),
            memo: None,
            action: Some(DepositAction::Execute(ExecuteIntents {
                execute_intents: vec![],
                refund_if_fails: true,
            })),
        };
        let json = serde_json::to_string(&msg).unwrap();

        // Should serialize with flattened execute fields
        assert!(json.contains("\"receiver_id\":\"alice.near\""));
        assert!(json.contains("\"execute_intents\""));
        assert!(json.contains("\"refund_if_fails\":true"));
    }

    #[test]
    fn test_serialize_with_memo() {
        // Simple message with memo serialization
        let mut msg = DepositMessage::new("alice.near".parse().unwrap());
        msg.memo = Some("new-memo-here".into());
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("\"receiver_id\":\"alice.near\""));
        assert!(json.contains("\"memo\":\"new-memo-here\""));
        assert!(!json.contains("action"));
        assert!(!json.contains("execute_intents"));
    }

    #[test]
    fn test_display_simple() {
        // Display for simple message (just account ID)
        let msg = DepositMessage::new("alice.near".parse().unwrap());
        assert_eq!(msg.to_string(), "alice.near");
    }

    // #[test]
    // fn test_display_with_action() {
    //     // Display for message with action (should be JSON)
    //     let msg = DepositMessage {
    //         receiver_id: "alice.near".parse().unwrap(),
    //         action: Some(DepositAction::Notify(NotifyOnTransfer::new(
    //             "test".to_string(),
    //         ))),
    //     };
    //     let display = msg.to_string();

    //     assert!(display.starts_with('{'));
    //     assert!(display.contains("alice.near"));
    // }

    #[test]
    fn test_from_str_simple() {
        // Parse simple account ID
        let msg: DepositMessage = "alice.near".parse().unwrap();
        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert!(msg.action.is_none());
    }

    #[test]
    fn test_from_str_json_with_execute() {
        // Parse JSON with execute intents and memo
        let json = r#"{"receiver_id":"alice.near","execute_intents":[],"memo":"the-memo","refund_if_fails":true}"#;
        let msg: DepositMessage = json.parse().unwrap();

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert_eq!(msg.memo.unwrap(), "the-memo");
        match msg.action {
            Some(DepositAction::Execute(exec)) => {
                assert!(exec.execute_intents.is_empty());
                assert!(exec.refund_if_fails);
            }
            _ => panic!("Expected Execute action"),
        }
    }

    // #[test]
    // fn test_from_str_json_with_notify() {
    //     // Parse JSON with notify action
    //     let json = r#"{"receiver_id":"alice.near","msg":"test"}"#;
    //     let msg: DepositMessage = json.parse().unwrap();

    //     assert_eq!(msg.receiver_id.as_str(), "alice.near");
    //     match msg.action {
    //         Some(DepositAction::Notify(notify)) => {
    //             assert_eq!(notify.msg, "test");
    //         }
    //         _ => panic!("Expected Notify action"),
    //     }
    // }

    // #[test]
    // fn test_deserialize_execute_takes_precedence_when_both_fields_present() {
    //     // When both execute_intents and msg are present, Execute variant should be matched first
    //     // since it comes first in the untagged enum
    //     let json = r#"{
    //         "receiver_id": "alice.near",
    //         "execute_intents": [],
    //         "refund_if_fails": true,
    //         "msg": "this should be ignored"
    //     }"#;
    //     let deposit_msg: DepositMessage = serde_json::from_str(json).unwrap();

    //     assert_eq!(deposit_msg.receiver_id.as_str(), "alice.near");
    //     match deposit_msg.action {
    //         Some(DepositAction::Execute(exec)) => {
    //             assert!(exec.execute_intents.is_empty());
    //             assert!(exec.refund_if_fails);
    //         }
    //         Some(DepositAction::Notify(_)) => {
    //             panic!("Expected Execute action, got Notify instead");
    //         }
    //         None => panic!("Expected Execute action, got None"),
    //     }
    // }

    #[test]
    fn test_builder_methods() {
        // Test direct construction
        let msg = DepositMessage {
            receiver_id: "alice.near".parse().unwrap(),
            memo: Some("the-memo".into()),
            action: Some(DepositAction::Execute(ExecuteIntents {
                execute_intents: vec![],
                refund_if_fails: true,
            })),
        };

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert_eq!(msg.memo.unwrap(), "the-memo");
        match msg.action {
            Some(DepositAction::Execute(exec)) => {
                assert!(exec.refund_if_fails);
            }
            _ => panic!("Expected Execute action"),
        }
    }

    // #[test]
    // fn test_builder_with_notify() {
    //     // Test direct construction with notify
    //     let msg = DepositMessage {
    //         receiver_id: "alice.near".parse().unwrap(),
    //         action: Some(DepositAction::Notify(NotifyOnTransfer::new(
    //             "test".to_string(),
    //         ))),
    //     };

    //     assert_eq!(msg.receiver_id.as_str(), "alice.near");
    //     assert!(matches!(msg.action, Some(DepositAction::Notify(_))));
    // }

    #[rstest::rstest]
    #[case::absent_is_default(None, Ok(DEPOSIT_MEMO))]
    #[case::empty_is_default(Some(""), Ok(DEPOSIT_MEMO))]
    #[case::explicit_default_allowed(Some(DEPOSIT_MEMO), Ok(DEPOSIT_MEMO))]
    #[case::custom(Some("AQ3f+enc/memo=="), Ok("AQ3f+enc/memo=="))]
    #[case::reserved_match_is_exact_case(Some("Refund"), Ok("Refund"))]
    #[case::reserved_match_is_exact_whitespace(Some("refund "), Ok("refund "))]
    #[case::reserved_refund(Some(REFUND_MEMO), Err(InvalidDepositMemo::Reserved(REFUND_MEMO.into())))]
    #[case::reserved_withdraw(Some("withdraw"), Err(InvalidDepositMemo::Reserved("withdraw".into())))]
    fn deposit_mint_memo_classes(
        #[case] memo: Option<&str>,
        #[case] expected: Result<&str, InvalidDepositMemo>,
    ) {
        assert_eq!(deposit_mint_memo(memo), expected);
    }

    #[test]
    fn deposit_mint_memo_length_is_capped_in_bytes() {
        let max = "m".repeat(MAX_DEPOSIT_MEMO_LEN);
        assert_eq!(deposit_mint_memo(Some(&max)), Ok(max.as_str()));

        let over = "m".repeat(MAX_DEPOSIT_MEMO_LEN + 1);
        assert_eq!(
            deposit_mint_memo(Some(&over)).unwrap_err(),
            InvalidDepositMemo::TooLong(MAX_DEPOSIT_MEMO_LEN + 1)
        );

        // 2-byte characters: 129 characters are 258 bytes, over the cap
        let over_multibyte = "é".repeat(MAX_DEPOSIT_MEMO_LEN / 2 + 1);
        assert_eq!(
            deposit_mint_memo(Some(&over_multibyte)).unwrap_err(),
            InvalidDepositMemo::TooLong(MAX_DEPOSIT_MEMO_LEN + 2)
        );
    }

    #[test]
    fn legacy_notify_shape_is_a_plain_deposit() {
        // the fields of the removed Notify action are ignored, and `memo` doesn't change that
        let msg: DepositMessage = r#"{"receiver_id":"alice.near","msg":"x","min_gas":"1"}"#
            .parse()
            .unwrap();
        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert!(msg.memo.is_none());
        assert!(msg.action.is_none());
    }

    #[test]
    fn display_with_empty_intents_is_bare_receiver() {
        let msg = DepositMessage::new("alice.near".parse().unwrap()).with_action(
            DepositAction::Execute(ExecuteIntents {
                execute_intents: vec![],
                refund_if_fails: true,
            }),
        );
        assert_eq!(msg.to_string(), "alice.near");
    }

    #[test]
    fn display_with_memo_is_json() {
        let msg = DepositMessage::new("alice.near".parse().unwrap()).with_memo("m");
        assert_eq!(
            msg.to_string(),
            r#"{"receiver_id":"alice.near","memo":"m"}"#
        );
    }

    #[test]
    fn display_with_memo_and_empty_intents_keeps_memo() {
        let msg = DepositMessage::new("alice.near".parse().unwrap())
            .with_memo("m")
            .with_action(DepositAction::Execute(ExecuteIntents {
                execute_intents: vec![],
                refund_if_fails: true,
            }));
        assert_eq!(
            msg.to_string(),
            r#"{"receiver_id":"alice.near","memo":"m","execute_intents":[],"refund_if_fails":true}"#
        );
    }

    #[test]
    fn from_str_rejects_invalid_memo() {
        let s = r#"{"receiver_id":"alice.near","memo":"withdraw","execute_intents":[]}"#;
        match s.parse::<DepositMessage>() {
            Err(ParseDepositMessageError::Memo(err)) => {
                assert_eq!(err, InvalidDepositMemo::Reserved("withdraw".into()));
            }
            other => panic!("expected a memo error, got {other:?}"),
        }
    }
}
