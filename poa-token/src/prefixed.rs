use std::marker::PhantomData;

pub trait MessagePrefix {
    const PREFIX: &'static str;
}

pub enum PrefixedMessage<'a, M> {
    NoMatch(&'a str),
    Matched {
        suffix: &'a str,
        rest: &'a str,
        _marker: PhantomData<M>,
    },
}

impl<'a, M> PrefixedMessage<'a, M> {
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

impl<'a, M: MessagePrefix> From<&'a str> for PrefixedMessage<'a, M> {
    fn from(msg: &'a str) -> Self {
        let msg_parts = msg.splitn(3, ':').collect::<Vec<_>>();
        if msg_parts.len() >= 2 && msg_parts[0] == M::PREFIX {
            let receiver_from_msg_str = msg_parts[1];
            let rest_of_the_message = msg_parts.get(2).unwrap_or(&"");

            Self::Matched {
                suffix: receiver_from_msg_str,
                rest: &rest_of_the_message,
                _marker: PhantomData,
            }
        } else {
            Self::NoMatch(msg)
        }
    }
}

// FIXME: add tests
