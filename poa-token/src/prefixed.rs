use near_sdk::AccountId;
use std::{fmt::Display, str::FromStr};

pub const UNWRAP_PREFIX: &str = "UNWRAP_TO";

pub enum PrefixedMessage<'a> {
    NoMatch(&'a str),
    MatchedNoMessage(AccountId),
    Matched {
        account_id: AccountId,
        rest: &'a str,
    },
}

impl PrefixedMessage<'_> {
    #[must_use]
    pub fn rest_of_the_message(&self) -> &str {
        match self {
            PrefixedMessage::NoMatch(msg) => msg,
            PrefixedMessage::Matched {
                account_id: _,
                rest,
            } => rest,
            PrefixedMessage::MatchedNoMessage(_account_id) => "",
        }
    }

    #[must_use]
    pub fn make_message_str(&self) -> String {
        match self {
            PrefixedMessage::NoMatch(m) => (*m).to_string(),
            PrefixedMessage::MatchedNoMessage(account_id) => {
                format!("{UNWRAP_PREFIX}:{account_id}")
            }
            PrefixedMessage::Matched { account_id, rest } => {
                format!("{UNWRAP_PREFIX}:{account_id}:{rest}")
            }
        }
    }
}

impl Display for PrefixedMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.make_message_str().fmt(f)
    }
}

impl<'a> From<&'a str> for PrefixedMessage<'a> {
    fn from(msg: &'a str) -> Self {
        let msg_parts = msg.splitn(3, ':').collect::<Vec<_>>();
        if msg_parts.len() >= 2 && msg_parts[0] == UNWRAP_PREFIX {
            let suffix = msg_parts[1];
            let rest_of_the_message = msg_parts.get(2).unwrap_or(&"");

            match AccountId::from_str(suffix) {
                Ok(s) => Self::Matched {
                    account_id: s,
                    rest: rest_of_the_message,
                },
                Err(_) => Self::NoMatch(msg),
            }
        } else {
            Self::NoMatch(msg)
        }
    }
}

// FIXME: add tests
