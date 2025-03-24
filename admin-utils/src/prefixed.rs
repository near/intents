use std::{fmt::Display, marker::PhantomData, str::FromStr};

pub trait MessagePrefix {
    const PREFIX: &'static str;
}

pub enum PrefixedMessage<'a, M, A> {
    NoMatch(&'a str),
    Matched {
        suffix: A,
        rest: &'a str,
        _marker: PhantomData<M>,
    },
}

impl<M, A> PrefixedMessage<'_, M, A> {
    #[must_use]
    pub fn rest_of_the_message(&self) -> &str {
        match self {
            PrefixedMessage::NoMatch(msg) => msg,
            PrefixedMessage::Matched {
                suffix: _,
                rest,
                _marker,
            } => rest,
        }
    }
}

impl<M: MessagePrefix, A> PrefixedMessage<'_, M, A> {
    #[must_use]
    pub fn make_message_str(
        suffix: impl AsRef<str>,
        rest_of_the_message: impl AsRef<str>,
    ) -> String {
        format!(
            "{}:{}:{}",
            M::PREFIX,
            suffix.as_ref(),
            rest_of_the_message.as_ref()
        )
    }
}

impl<M: MessagePrefix, A: ToString> Display for PrefixedMessage<'_, M, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrefixedMessage::NoMatch(m) => m.fmt(f),
            PrefixedMessage::Matched {
                suffix,
                rest,
                _marker,
            } => Self::make_message_str(suffix.to_string(), rest).fmt(f),
        }
    }
}

impl<'a, M: MessagePrefix, A: FromStr> From<&'a str> for PrefixedMessage<'a, M, A> {
    fn from(msg: &'a str) -> Self {
        let msg_parts = msg.splitn(3, ':').collect::<Vec<_>>();
        if msg_parts.len() >= 2 && msg_parts[0] == M::PREFIX {
            let suffix = msg_parts[1];
            let rest_of_the_message = msg_parts.get(2).unwrap_or(&"");

            match A::from_str(suffix) {
                Ok(s) => Self::Matched {
                    suffix: s,
                    rest: rest_of_the_message,
                    _marker: PhantomData,
                },
                Err(_) => Self::NoMatch(msg),
            }
        } else {
            Self::NoMatch(msg)
        }
    }
}

// FIXME: add tests
