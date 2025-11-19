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

#[near(serializers = [json])]
#[serde(untagged)]
#[derive(Debug, Clone)]
pub enum DepositMessage {
    V1(DepositMessageV1),
    V2(DepositMessageV2),
}

#[near(serializers = [json])]
#[serde(deny_unknown_fields)]
#[derive(Debug, Clone)]
pub struct DepositMessageV1 {
    pub receiver_id: AccountId,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub execute_intents: Vec<MultiPayload>,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub refund_if_fails: bool,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct DepositMessageV2 {
    pub receiver_id: AccountId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<DepositMessageActionV2>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct ExecuteIntents {
    pub execute_intents: Vec<MultiPayload>,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub refund_if_fails: bool,
}

#[near(serializers = [json])]
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum DepositMessageActionV2 {
    Notify(NotifyOnTransfer),
    Execute(ExecuteIntents),
}

impl From<DepositMessageV1> for DepositMessageV2 {
    fn from(v1: DepositMessageV1) -> Self {
        let action = if !v1.execute_intents.is_empty() || v1.refund_if_fails {
            Some(DepositMessageActionV2::Execute(ExecuteIntents {
                execute_intents: v1.execute_intents,
                refund_if_fails: v1.refund_if_fails,
            }))
        } else {
            None
        };

        Self {
            receiver_id: v1.receiver_id,
            action,
        }
    }
}

impl DepositMessage {
    #[must_use]
    #[inline]
    pub const fn new(receiver_id: AccountId) -> Self {
        Self::V2(DepositMessageV2 {
            receiver_id,
            action: None,
        })
    }

    #[must_use]
    pub fn into_v2(self) -> DepositMessageV2 {
        match self {
            Self::V2(v2) => v2,
            Self::V1(v1) => v1.into(),
        }
    }

    #[must_use]
    pub fn receiver_id(&self) -> &AccountId {
        match self {
            Self::V2(v2) => &v2.receiver_id,
            Self::V1(v1) => &v1.receiver_id,
        }
    }
}

// V1 Builder methods
impl DepositMessageV1 {
    #[must_use]
    #[inline]
    pub const fn new(receiver_id: AccountId) -> Self {
        Self {
            receiver_id,
            execute_intents: Vec::new(),
            refund_if_fails: false,
        }
    }

    #[must_use]
    #[inline]
    pub fn with_execute_intents(mut self, intents: impl IntoIterator<Item = MultiPayload>) -> Self {
        self.execute_intents.extend(intents);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_refund_if_fails(mut self) -> Self {
        self.refund_if_fails = true;
        self
    }
}

// V2 Builder methods
impl DepositMessageV2 {
    #[must_use]
    #[inline]
    pub const fn new(receiver_id: AccountId) -> Self {
        Self {
            receiver_id,
            action: None,
        }
    }

    #[must_use]
    #[inline]
    pub fn with_execute_intents(mut self, intents: impl IntoIterator<Item = MultiPayload>) -> Self {
        self.action = Some(DepositMessageActionV2::Execute(ExecuteIntents {
            execute_intents: intents.into_iter().collect(),
            refund_if_fails: false,
        }));
        self
    }

    #[must_use]
    #[inline]
    pub fn with_refund_if_fails(mut self) -> Self {
        if let Some(DepositMessageActionV2::Execute(ref mut exec)) = self.action {
            exec.refund_if_fails = true;
        }
        self
    }

    #[must_use]
    #[inline]
    pub fn with_notify(mut self, notify: NotifyOnTransfer) -> Self {
        self.action = Some(DepositMessageActionV2::Notify(notify));
        self
    }

    #[must_use]
    #[inline]
    pub fn with_action(mut self, action: DepositMessageActionV2) -> Self {
        self.action = Some(action);
        self
    }
}

impl Display for DepositMessage {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V2(v2) => match &v2.action {
                None => f.write_str(v2.receiver_id.as_str()),
                Some(DepositMessageActionV2::Execute(exec)) if exec.execute_intents.is_empty() => {
                    f.write_str(v2.receiver_id.as_str())
                }
                Some(_) => f.write_str(&serde_json::to_string(self).unwrap_or_panic_display()),
            },
            Self::V1(v1) => {
                if v1.execute_intents.is_empty() && !v1.refund_if_fails {
                    f.write_str(v1.receiver_id.as_str())
                } else {
                    f.write_str(&serde_json::to_string(self).unwrap_or_panic_display())
                }
            }
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
    fn test_v1_deserialize_simple() {
        // V1 format: just receiver_id
        let json = r#"{"receiver_id": "alice.near"}"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        match msg {
            DepositMessage::V1(v1) => {
                assert_eq!(v1.receiver_id.as_str(), "alice.near");
                assert!(v1.execute_intents.is_empty());
                assert!(!v1.refund_if_fails);
            }
            DepositMessage::V2(_) => panic!("Expected V1"),
        }
    }

    #[test]
    fn test_v1_deserialize_with_execute_intents() {
        // V1 format: with execute_intents
        let json = r#"{
            "receiver_id": "alice.near",
            "execute_intents": [],
            "refund_if_fails": true
        }"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        match msg {
            DepositMessage::V1(v1) => {
                assert_eq!(v1.receiver_id.as_str(), "alice.near");
                assert!(v1.execute_intents.is_empty());
                assert!(v1.refund_if_fails);
            }
            DepositMessage::V2(_) => panic!("Expected V1"),
        }
    }

    #[test]
    fn test_v2_deserialize_simple() {
        // V2 format: just receiver_id with no action
        let json = r#"{"receiver_id": "alice.near"}"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        // This should deserialize as V1 due to untagged enum order
        match msg {
            DepositMessage::V1(v1) => {
                assert_eq!(v1.receiver_id.as_str(), "alice.near");
            }
            DepositMessage::V2(_) => panic!("Expected V1 for simple format"),
        }
    }

    #[test]
    fn test_v2_deserialize_with_notify() {
        // V2 format: with notify action (tagged)
        let json = r#"{
            "receiver_id": "alice.near",
            "action": {
                "type": "notify",
                "msg": "hello world",
                "min_gas": null
            }
        }"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        match msg {
            DepositMessage::V2(v2) => {
                assert_eq!(v2.receiver_id.as_str(), "alice.near");
                match v2.action {
                    Some(DepositMessageActionV2::Notify(notify)) => {
                        assert_eq!(notify.msg, "hello world");
                        assert!(notify.min_gas.is_none());
                    }
                    _ => panic!("Expected Notify action"),
                }
            }
            DepositMessage::V1(_) => panic!("Expected V2"),
        }
    }

    #[test]
    fn test_v2_deserialize_with_execute() {
        // V2 format: with execute action (tagged)
        let json = r#"{
            "receiver_id": "alice.near",
            "action": {
                "type": "execute",
                "execute_intents": [],
                "refund_if_fails": true
            }
        }"#;
        let msg: DepositMessage = serde_json::from_str(json).unwrap();

        match msg {
            DepositMessage::V2(v2) => {
                assert_eq!(v2.receiver_id.as_str(), "alice.near");
                match v2.action {
                    Some(DepositMessageActionV2::Execute(exec)) => {
                        assert!(exec.execute_intents.is_empty());
                        assert!(exec.refund_if_fails);
                    }
                    _ => panic!("Expected Execute action"),
                }
            }
            DepositMessage::V1(_) => panic!("Expected V2"),
        }
    }

    #[test]
    fn test_v1_to_v2_conversion_empty() {
        // V1 with no intents converts to V2 with no action
        let v1 = DepositMessageV1::new("alice.near".parse().unwrap());
        let v2: DepositMessageV2 = v1.into();

        assert_eq!(v2.receiver_id.as_str(), "alice.near");
        assert!(v2.action.is_none());
    }

    #[test]
    fn test_v1_to_v2_conversion_with_intents() {
        // V1 with intents converts to V2 with Execute action
        let v1 = DepositMessageV1 {
            receiver_id: "alice.near".parse().unwrap(),
            execute_intents: vec![],
            refund_if_fails: true,
        };
        let v2: DepositMessageV2 = v1.into();

        assert_eq!(v2.receiver_id.as_str(), "alice.near");
        match v2.action {
            Some(DepositMessageActionV2::Execute(exec)) => {
                assert!(exec.execute_intents.is_empty());
                assert!(exec.refund_if_fails);
            }
            _ => panic!("Expected Execute action"),
        }
    }

    #[test]
    fn test_into_v2_enum_v1() {
        // DepositMessage::V1 converts to V2
        let msg = DepositMessage::V1(DepositMessageV1 {
            receiver_id: "alice.near".parse().unwrap(),
            execute_intents: vec![],
            refund_if_fails: true,
        });

        let v2 = msg.into_v2();
        assert_eq!(v2.receiver_id.as_str(), "alice.near");
        assert!(matches!(v2.action, Some(DepositMessageActionV2::Execute(_))));
    }

    #[test]
    fn test_into_v2_enum_v2() {
        // DepositMessage::V2 returns itself
        let v2_original = DepositMessageV2::new("alice.near".parse().unwrap())
            .with_notify(NotifyOnTransfer {
                msg: "test".to_string(),
                min_gas: None,
            });

        let msg = DepositMessage::V2(v2_original.clone());
        let v2 = msg.into_v2();

        assert_eq!(v2.receiver_id.as_str(), "alice.near");
        assert!(matches!(v2.action, Some(DepositMessageActionV2::Notify(_))));
    }

    #[test]
    fn test_serialize_v1() {
        // V1 serialization
        let v1 = DepositMessageV1 {
            receiver_id: "alice.near".parse().unwrap(),
            execute_intents: vec![],
            refund_if_fails: true,
        };
        let msg = DepositMessage::V1(v1);
        let json = serde_json::to_string(&msg).unwrap();

        // Should serialize as flat V1 format
        assert!(json.contains("\"receiver_id\":\"alice.near\""));
        assert!(json.contains("\"refund_if_fails\":true"));
    }

    #[test]
    fn test_serialize_v2() {
        // V2 serialization with tagged action
        let v2 = DepositMessageV2::new("alice.near".parse().unwrap())
            .with_notify(NotifyOnTransfer {
                msg: "hello".to_string(),
                min_gas: None,
            });
        let msg = DepositMessage::V2(v2);
        let json = serde_json::to_string(&msg).unwrap();

        // Should serialize with tagged action
        assert!(json.contains("\"receiver_id\":\"alice.near\""));
        assert!(json.contains("\"type\":\"notify\""));
        assert!(json.contains("\"msg\":\"hello\""));
    }

    #[test]
    fn test_display_simple() {
        // Display for simple message (just account ID)
        let msg = DepositMessage::new("alice.near".parse().unwrap());
        assert_eq!(msg.to_string(), "alice.near");
    }

    #[test]
    fn test_display_v1_with_intents() {
        // Display for V1 with intents (should be JSON)
        let v1 = DepositMessageV1 {
            receiver_id: "alice.near".parse().unwrap(),
            execute_intents: vec![],
            refund_if_fails: true,
        };
        let msg = DepositMessage::V1(v1);
        let display = msg.to_string();

        assert!(display.starts_with('{'));
        assert!(display.contains("alice.near"));
    }

    #[test]
    fn test_from_str_simple() {
        // Parse simple account ID
        let msg: DepositMessage = "alice.near".parse().unwrap();
        assert_eq!(msg.receiver_id().as_str(), "alice.near");
    }

    #[test]
    fn test_from_str_json_v1() {
        // Parse V1 JSON
        let json = r#"{"receiver_id":"alice.near","refund_if_fails":true}"#;
        let msg: DepositMessage = json.parse().unwrap();

        match msg {
            DepositMessage::V1(v1) => {
                assert_eq!(v1.receiver_id.as_str(), "alice.near");
                assert!(v1.refund_if_fails);
            }
            DepositMessage::V2(_) => panic!("Expected V1"),
        }
    }

    #[test]
    fn test_from_str_json_v2() {
        // Parse V2 JSON
        let json = r#"{"receiver_id":"alice.near","action":{"type":"notify","msg":"test"}}"#;
        let msg: DepositMessage = json.parse().unwrap();

        match msg {
            DepositMessage::V2(v2) => {
                assert_eq!(v2.receiver_id.as_str(), "alice.near");
                assert!(matches!(v2.action, Some(DepositMessageActionV2::Notify(_))));
            }
            DepositMessage::V1(_) => panic!("Expected V2"),
        }
    }

    #[test]
    fn test_receiver_id_helper() {
        // Test receiver_id() helper works for both V1 and V2
        let v1_msg = DepositMessage::V1(DepositMessageV1::new("alice.near".parse().unwrap()));
        assert_eq!(v1_msg.receiver_id().as_str(), "alice.near");

        let v2_msg = DepositMessage::V2(DepositMessageV2::new("bob.near".parse().unwrap()));
        assert_eq!(v2_msg.receiver_id().as_str(), "bob.near");
    }

    #[test]
    fn test_v1_builder_methods() {
        // Test V1 builder pattern
        let msg = DepositMessageV1::new("alice.near".parse().unwrap())
            .with_execute_intents(vec![])
            .with_refund_if_fails();

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        assert!(msg.refund_if_fails);
    }

    #[test]
    fn test_v2_builder_methods() {
        // Test V2 builder pattern
        let msg = DepositMessageV2::new("alice.near".parse().unwrap())
            .with_execute_intents(vec![])
            .with_refund_if_fails();

        assert_eq!(msg.receiver_id.as_str(), "alice.near");
        match msg.action {
            Some(DepositMessageActionV2::Execute(exec)) => {
                assert!(exec.refund_if_fails);
            }
            _ => panic!("Expected Execute action"),
        }
    }
}
