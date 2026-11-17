use defuse_crypto::ed25519::Ed25519PublicKey;
use defuse_wallet::{RequestMessage, offchain::OffchainMessage, readable_message};
use serde_json::{Map, Value, json};

pub use defuse_wallet::readable_message::{
    MAX_SIGNING_MESSAGE_BYTES, OFFCHAIN_PREFIX, REQUEST_PREFIX,
};

/// A message cannot be signed by this schema.
#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error(transparent)]
    Readable(#[from] readable_message::MessageError),
    #[error("text must be ASCII without DEL")]
    Text,
    #[error("a wallet would show the message differently than it is signed")]
    Display,
    #[error("signing message exceeds 16 KiB")]
    TooLarge,
}

/// Stellar account ID of the key: `G...` strkey.
pub fn stellar_address(public_key: &Ed25519PublicKey) -> String {
    stellar_strkey::ed25519::PublicKey(public_key.0)
        .to_string()
        .as_str()
        .to_owned()
}

/// Reconstruct the readable v1 execution message for this Stellar key.
pub fn request_signing_message(
    public_key: &Ed25519PublicKey,
    msg: &RequestMessage,
) -> Result<String, MessageError> {
    let address = stellar_address(public_key);
    let text = readable_message::request_signing_message(&[("stellar_address", &address)], msg)?;
    document(REQUEST_PREFIX, &text)
}

/// Reconstruct all five NEP-641 fields, preserving the exact payload string.
pub fn offchain_signing_message(
    public_key: &Ed25519PublicKey,
    msg: &OffchainMessage,
) -> Result<String, MessageError> {
    let address = stellar_address(public_key);
    let text = readable_message::offchain_signing_message(&[("stellar_address", &address)], msg)?;
    document(OFFCHAIN_PREFIX, &text)
}

/// The readable v1 `text` as one JSON document: its prefix lines in `about`,
/// its warning lines in `warnings`, then its JSON fields.
///
/// The document matches the modeled Freighter JSON serialization:
/// `JSON.stringify(JSON.parse(message), null, 2) + "\n"`, encoded as UTF-8.
/// The final LF is signed. This models serialization, not the actual wallet
/// display; real-wallet display verification remains an integration check.
/// See the variant README for the inspected Freighter source revision.
fn document(prefix: &str, text: &str) -> Result<String, MessageError> {
    let rest = text.strip_prefix(prefix).ok_or(MessageError::Display)?;
    // Warning lines, one empty line, then the JSON.
    let json = if rest.starts_with("{\n") {
        0
    } else {
        rest.find("\n\n{\n").ok_or(MessageError::Display)? + 2
    };
    let (warnings, json) = rest.split_at(json);
    let fields: Map<String, Value> =
        serde_json::from_str(json).map_err(|_| MessageError::Display)?;
    let mut document = Map::new();
    document.insert("about".to_owned(), json!(lines(prefix)));
    document.insert("warnings".to_owned(), json!(lines(warnings)));
    for (name, value) in fields {
        if document.insert(name, value).is_some() {
            return Err(MessageError::Display);
        }
    }
    let document = Value::Object(document);
    shown_as_signed(&document)?;
    let mut message =
        serde_json::to_string_pretty(&document).map_err(readable_message::MessageError::from)?;
    message.push('\n');
    if message.len() > MAX_SIGNING_MESSAGE_BYTES {
        return Err(MessageError::TooLarge);
    }
    Ok(message)
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().filter(|line| !line.is_empty()).collect()
}

/// Largest integer that JavaScript numbers represent exactly (2^53 - 1).
const MAX_SAFE_INTEGER: i64 = (1 << 53) - 1;

/// Whether `JSON.stringify(JSON.parse(..), null, 2)` returns the pretty JSON of
/// `value` unchanged:
///
/// - text is ASCII without DEL, so the readable format has no `\uXXXX` escape
///   that JavaScript would show as the character itself (bidirectional
///   controls, look-alike letters);
/// - numbers are integers within ±(2^53 - 1);
/// - objects list array-index keys first, in ascending order, as JavaScript
///   objects do.
fn shown_as_signed(value: &Value) -> Result<(), MessageError> {
    let text = |text: &str| {
        if text.bytes().all(|byte| byte < 0x7f) {
            Ok(())
        } else {
            Err(MessageError::Text)
        }
    };
    match value {
        Value::Null | Value::Bool(_) => Ok(()),
        Value::String(string) => text(string),
        Value::Number(number) => number
            .as_i64()
            .filter(|n| (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(n))
            .map(|_| ())
            .ok_or(MessageError::Display),
        Value::Array(items) => items.iter().try_for_each(shown_as_signed),
        Value::Object(object) => {
            if !object
                .keys()
                .is_sorted_by_key(|key| array_index(key).unwrap_or(u32::MAX))
            {
                return Err(MessageError::Display);
            }
            object.iter().try_for_each(|(key, value)| {
                text(key)?;
                shown_as_signed(value)
            })
        }
    }
}

/// JavaScript array index: a canonical decimal below 2^32 - 1.
fn array_index(key: &str) -> Option<u32> {
    key.parse()
        .ok()
        .filter(|index: &u32| *index != u32::MAX && index.to_string() == key)
}
