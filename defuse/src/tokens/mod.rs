pub mod nep141;
pub mod nep171;
pub mod nep245;

use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use defuse_core::{intents::tokens::NotifyOnTransfer, payload::multi::MultiPayload};
use defuse_near_utils::UnwrapOrPanicError;
use near_account_id::ParseAccountError;
use near_sdk::{AccountId, near, serde_json};
use thiserror::Error as ThisError;

#[must_use]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct DepositMessage {
    pub receiver_id: AccountId,

    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub action: Option<DepositAction>,
}

impl DepositMessage {
    #[inline]
    pub const fn new(receiver_id: AccountId) -> Self {
        Self {
            receiver_id,
            action: None,
        }
    }

    #[inline]
    pub fn with_action(mut self, action: impl Into<Option<DepositAction>>) -> Self {
        self.action = action.into();
        self
    }
}

impl Display for DepositMessage {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.action {
            None => f.write_str(self.receiver_id.as_str()),
            Some(DepositAction::Execute(exec)) if exec.execute_intents.is_empty() => {
                f.write_str(self.receiver_id.as_str())
            }
            Some(_) => f.write_str(&serde_json::to_string(self).unwrap_or_panic_display()),
        }
    }
}

impl FromStr for DepositMessage {
    type Err = ParseDepositMessageError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('{') {
            serde_json::from_str(s).map_err(Into::into)
        } else {
            s.parse().map(Self::new).map_err(Into::into)
        }
    }
}

#[must_use]
#[near(serializers = [json])]
#[serde(untagged)]
#[derive(Debug, Clone)]
pub enum DepositAction {
    Execute(ExecuteIntents),
    Notify(NotifyOnTransfer),
}

#[must_use]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct ExecuteIntents {
    pub execute_intents: Vec<MultiPayload>,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub refund_if_fails: bool,
}

#[derive(Debug, ThisError)]
pub enum ParseDepositMessageError {
    #[error(transparent)]
    Account(#[from] ParseAccountError),
    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use defuse_core::intents::tokens::NotifyOnTransfer;

    #[test]
    fn test_deserialize_simple() {
        // Simple format: just receiver_id
        let json = r#"{"receiver_id": "alice.near"}"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert!(msg.action.is_none());
    }

    #[test]
    fn test_deserialize_with_notify() {
        // With notify action (flattened untagged)
        let json = r#"{
            "receiver_id": "alice.near",
            "msg": "hello world",
            "min_gas": null
        }"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        match msg.action {
            Some(DepositAction::Notify(notify)) => {
                assert_eq!(notify.msg, "hello world");
                assert!(notify.min_gas.is_none());
            }
            _ => panic!("Expected Notify action"),
        }
    }

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
    fn test_serialize_simple() {
        // Simple message serialization
        let msg = DepositMessage::new("alice.near".parse().unwrap());
        let json = serde_json::to_string(&msg).unwrap();

        // Should serialize with just receiver_id (action omitted when None)
        assert!(json.contains("\"receiver_id\":\"alice.near\""));
        assert!(!json.contains("action"));
    }

    #[test]
    fn test_serialize_with_notify() {
        // Serialization with notify action
        let msg = DepositMessage {
            receiver_id: "alice.near".parse().unwrap(),
            action: Some(DepositAction::Notify(NotifyOnTransfer {
                msg: "hello".to_string(),
                min_gas: None,
            })),
        };
        let json = serde_json::to_string(&msg).unwrap();

        // Should serialize with flattened notify fields
        assert!(json.contains("\"receiver_id\":\"alice.near\""));
        assert!(json.contains("\"msg\":\"hello\""));
    }

    #[test]
    fn test_serialize_with_execute() {
        // Serialization with execute action
        let msg = DepositMessage {
            receiver_id: "alice.near".parse().unwrap(),
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
    fn test_display_simple() {
        // Display for simple message (just account ID)
        let msg = DepositMessage::new("alice.near".parse().unwrap());
        assert_eq!(msg.to_string(), "alice.near");
    }

    #[test]
    fn test_display_with_action() {
        // Display for message with action (should be JSON)
        let msg = DepositMessage {
            receiver_id: "alice.near".parse().unwrap(),
            action: Some(DepositAction::Notify(NotifyOnTransfer {
                msg: "test".to_string(),
                min_gas: None,
            })),
        };
        let display = msg.to_string();

        assert!(display.starts_with('{'));
        assert!(display.contains("alice.near"));
    }

    #[test]
    fn test_from_str_simple() {
        // Parse simple account ID
        let msg: DepositMessage = "alice.near".parse().unwrap();
        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert!(msg.action.is_none());
    }

    #[test]
    fn test_from_str_json_with_execute() {
        // Parse JSON with execute intents
        let json = r#"{"receiver_id":"alice.near","execute_intents":[],"refund_if_fails":true}"#;
        let msg: DepositMessage = json.parse().unwrap();

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
    fn test_from_str_json_with_notify() {
        // Parse JSON with notify action
        let json = r#"{"receiver_id":"alice.near","msg":"test"}"#;
        let msg: DepositMessage = json.parse().unwrap();

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        match msg.action {
            Some(DepositAction::Notify(notify)) => {
                assert_eq!(notify.msg, "test");
            }
            _ => panic!("Expected Notify action"),
        }
    }

    #[test]
    fn test_deserialize_execute_takes_precedence_when_both_fields_present() {
        // When both execute_intents and msg are present, Execute variant should be matched first
        // since it comes first in the untagged enum
        let json = r#"{
            "receiver_id": "alice.near",
            "execute_intents": [],
            "refund_if_fails": true,
            "msg": "this should be ignored"
        }"#;
        let deposit_msg: DepositMessage = serde_json::from_str(json).unwrap();

        assert_eq!(deposit_msg.receiver_id.as_str(), "alice.near");
        match deposit_msg.action {
            Some(DepositAction::Execute(exec)) => {
                assert!(exec.execute_intents.is_empty());
                assert!(exec.refund_if_fails);
            }
            Some(DepositAction::Notify(_)) => {
                panic!("Expected Execute action, got Notify instead");
            }
            None => panic!("Expected Execute action, got None"),
        }
    }

    #[test]
    fn test_builder_methods() {
        // Test direct construction
        let msg = DepositMessage {
            receiver_id: "alice.near".parse().unwrap(),
            action: Some(DepositAction::Execute(ExecuteIntents {
                execute_intents: vec![],
                refund_if_fails: true,
            })),
        };

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        match msg.action {
            Some(DepositAction::Execute(exec)) => {
                assert!(exec.refund_if_fails);
            }
            _ => panic!("Expected Execute action"),
        }
    }

    #[test]
    fn test_builder_with_notify() {
        // Test direct construction with notify
        let msg = DepositMessage {
            receiver_id: "alice.near".parse().unwrap(),
            action: Some(DepositAction::Notify(NotifyOnTransfer {
                msg: "test".to_string(),
                min_gas: None,
            })),
        };

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert!(matches!(msg.action, Some(DepositAction::Notify(_))));
    }
}
