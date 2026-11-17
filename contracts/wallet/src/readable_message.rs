//! Deterministic readable v1 messages shared by wallet signing schemas.

use std::fmt::Write;

use crate::{
    AccountId, NearPromise, NearToken, Request, RequestMessage, StateInit, StateInitV1, WalletOp,
    actions::{DeterministicStateInit, FunctionCall, NearAction, Transfer},
    offchain::OffchainMessage,
};
use serde_json::{Map, Value, json};

/// Maximum UTF-8 message size accepted by this schema (16 KiB).
pub const MAX_SIGNING_MESSAGE_BYTES: usize = 16 * 1024;
/// Domain and purpose separator for execution requests.
pub const REQUEST_PREFIX: &str = "NEAR Wallet Contract: execute request (v1)\nThis signature authorizes NEAR wallet operations, not a login.\nGas is in gas units.\nFunction arguments and deployment state may be opaque: verify them before signing.\n\n";
/// Domain and purpose separator for NEP-641 authorizations.
pub const OFFCHAIN_PREFIX: &str = "NEAR Wallet Contract: offchain authorization (v1)\nThis signature authorizes the exact payload below, not a NEAR transaction.\nThe consuming application defines expiration and one-time-use requirements.\n\n";

/// A message cannot be represented by this schema.
#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("unsupported action")]
    UnsupportedAction,
    #[error("function call arguments must be compact JSON")]
    FunctionArgs,
    #[error("NEP-141 transfers need one require_ft_metadata operation for their token")]
    TokenMetadata,
    #[error("NEP-141 transfer amount must be a decimal u128 string")]
    TokenAmount,
    #[error("NEP-141 transfer arguments must be parameters of the method")]
    TokenArgs,
    #[error("timestamp must fit unsigned 64-bit Unix nanoseconds")]
    Timestamp,
    #[error("timeout must be a whole number of seconds fitting u32")]
    Timeout,
    #[error("signing identity field collides with a message field")]
    IdentityField,
    #[error("signing message exceeds 16 KiB")]
    TooLarge,
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

/// Top-level `(name, value)` fields that a schema reconstructs from its stored
/// public key, e.g. `[("sui_public_key", ..), ("sui_address", ..)]`. They follow
/// `signer_id`; names must not collide with message fields.
pub type SigningIdentity<'a> = &'a [(&'a str, &'a str)];

/// Reconstruct the full readable execution message.
///
/// Fields have a fixed order: who signs, then wallet operations before external
/// calls. Each `add_extension`, and each promise whose attached NEAR can be
/// refunded to an account other than the executing wallet, adds a warning line
/// before the JSON.
pub fn request_signing_message(
    identity: SigningIdentity<'_>,
    msg: &RequestMessage,
) -> Result<String, MessageError> {
    // No `..` when destructuring rendered types: a new field fails to compile
    // until it is rendered. Wallet operations and state init are rendered with
    // serde, which shows each of their serialized fields.
    let RequestMessage {
        pay_for_gas,
        chain_id,
        signer_id,
        nonce,
        created_at,
        timeout,
        request,
    } = msg;
    u64::try_from(created_at.as_nanos()).map_err(|_| MessageError::Timestamp)?;
    let timeout_secs = u32::try_from(timeout.as_secs()).map_err(|_| MessageError::Timeout)?;
    if timeout.subsec_nanos() != 0 {
        return Err(MessageError::Timeout);
    }
    // Before parsing any arguments, so oversized requests are rejected cheaply.
    if input_bytes(request) > MAX_SIGNING_MESSAGE_BYTES {
        return Err(MessageError::TooLarge);
    }
    let mut warnings = Vec::new();
    let request = request_value(request, signer_id, "this wallet", &mut warnings)?;
    render(
        REQUEST_PREFIX,
        &warnings,
        identity,
        [
            ("chain_id", json!(chain_id)),
            ("signer_id", json!(signer_id)),
        ],
        [
            ("nonce", json!(nonce)),
            ("created_at", json!(created_at)),
            ("timeout_secs", json!(timeout_secs)),
            ("pay_for_gas", json!(pay_for_gas)),
            ("request", request),
        ],
    )
}

/// Reconstruct the exact bytes for all five NEP-641 message fields.
///
/// The payload is an opaque string: its bytes are never parsed or normalized.
pub fn offchain_signing_message(
    identity: SigningIdentity<'_>,
    msg: &OffchainMessage,
) -> Result<String, MessageError> {
    let OffchainMessage {
        chain_id,
        signer_id,
        path,
        timestamp,
        payload,
    } = msg;
    u64::try_from(timestamp.as_nanos()).map_err(|_| MessageError::Timestamp)?;
    // The payload and each path account are shown at least once.
    let shown = path
        .iter()
        .map(|account| account.len())
        .fold(payload.len(), usize::saturating_add);
    if shown > MAX_SIGNING_MESSAGE_BYTES {
        return Err(MessageError::TooLarge);
    }
    render(
        OFFCHAIN_PREFIX,
        &[],
        identity,
        [
            ("chain_id", json!(chain_id)),
            ("signer_id", json!(signer_id)),
        ],
        [
            ("path", json!(path)),
            ("timestamp", json!(timestamp)),
            ("payload", json!(payload)),
        ],
    )
}

/// Each operation, promise and action renders to more bytes than this.
const MIN_ITEM_BYTES: usize = 64;

/// Size checked before rendering: raw function-call arguments, state-init data,
/// token symbols and [`MIN_ITEM_BYTES`] per operation, promise and action. It is
/// at most the rendered size, except that a nested request counts with its
/// base64 encoding, so some nested requests that would fit are rejected.
fn input_bytes(request: &Request) -> usize {
    let actions = request.external.iter().flat_map(|promise| &promise.actions);
    let items = request.internal.len() + request.external.len() + actions.clone().count();
    // Also repeated in each transfer amount of the token.
    let symbols = request.internal.iter().map(|op| match op {
        WalletOp::RequireFtMetadata { symbol, .. } => symbol.len(),
        _ => 0,
    });
    actions
        .map(|action| match action {
            NearAction::FunctionCall(FunctionCall { args, .. }) => args.len(),
            NearAction::DeterministicStateInit(DeterministicStateInit {
                state_init: StateInit::V1(StateInitV1 { data, .. }),
                ..
            }) => data
                .iter()
                .map(|(key, value)| key.len() + value.len())
                .sum(),
            _ => 0,
        })
        .chain(symbols)
        .fold(items.saturating_mul(MIN_ITEM_BYTES), usize::saturating_add)
}

/// Method through which this wallet acts as an extension of another wallet.
/// Its `request` argument is shown by the same rules as the signed request.
const EXECUTE_EXTENSION: &str = "w_execute_extension";

/// `w_execute_extension()` arguments.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtensionArgs {
    request: Request,
}

/// The request in `w_execute_extension()` arguments, which must be exactly its
/// serde serialization, so the message determines the argument bytes.
fn extension_request(args: &[u8]) -> Result<Request, MessageError> {
    let extension: ExtensionArgs =
        serde_json::from_value(args_value(args)?).map_err(|_| MessageError::FunctionArgs)?;
    if serde_json::to_vec(&extension)? != args {
        return Err(MessageError::FunctionArgs);
    }
    Ok(extension.request)
}

/// Wallet operations first, then external promises. `wallet` is the account
/// that executes `request`, and `name` refers to it in warnings.
fn request_value(
    request: &Request,
    wallet: &AccountId,
    name: &str,
    warnings: &mut Vec<String>,
) -> Result<Value, MessageError> {
    let Request { internal, external } = request;
    let tokens = tokens(internal)?;
    for op in internal {
        if let WalletOp::AddExtension { account_id } = op {
            warnings.push(format!(
                "WARNING: add_extension gives {account_id} full control of {name}."
            ));
        }
    }
    let external = external
        .iter()
        .map(|promise| promise_value(promise, (wallet, name), &tokens, warnings))
        .collect::<Result<_, _>>()?;
    Ok(Value::Object(object([
        ("internal", json!(internal)),
        ("external", Value::Array(external)),
    ])))
}

/// NEP-141 methods whose `amount` argument is shown with the token's decimals
/// and symbol, with their parameters: other arguments, which the token would
/// ignore, could imitate a shown amount and are rejected.
const FT_TRANSFER_METHODS: [(&str, &[&str]); 2] = [
    ("ft_transfer", &["receiver_id", "amount", "memo"]),
    (
        "ft_transfer_call",
        &["receiver_id", "amount", "memo", "msg"],
    ),
];

/// Decimals and symbol of each token from `require_ft_metadata` operations,
/// which the contract checks against `ft_metadata()` before the transfer.
type Tokens<'a> = Vec<(&'a AccountId, u8, &'a str)>;

fn tokens(internal: &[WalletOp]) -> Result<Tokens<'_>, MessageError> {
    let mut tokens = Tokens::new();
    for op in internal {
        if let WalletOp::RequireFtMetadata {
            token_id,
            decimals,
            symbol,
        } = op
        {
            if tokens.iter().any(|(other, ..)| *other == token_id) {
                return Err(MessageError::TokenMetadata);
            }
            tokens.push((token_id, *decimals, symbol));
        }
    }
    Ok(tokens)
}

/// `wallet` executes the promise; `refund_to` (`null` means `wallet`) receives
/// its failed and unused deposits, e.g. the unused part of a state-init budget.
fn promise_value(
    promise: &NearPromise,
    (wallet, name): (&AccountId, &str),
    tokens: &Tokens<'_>,
    warnings: &mut Vec<String>,
) -> Result<Value, MessageError> {
    let NearPromise {
        receiver_id,
        refund_to,
        actions,
    } = promise;
    if let Some(refund_to) = refund_to.as_ref().filter(|account| *account != wallet) {
        let attached = attached_near(promise);
        if attached != 0 {
            warnings.push(format!(
                "WARNING: up to {} sent to {receiver_id} can be refunded to {refund_to}, not to {name}.",
                near_amount(attached)
            ));
        }
    }
    let token = tokens
        .iter()
        .find(|(token_id, ..)| *token_id == receiver_id)
        .map(|&(_, decimals, symbol)| (decimals, symbol));
    let actions = actions
        .iter()
        .map(|action| action_value(action, receiver_id, token, warnings))
        .collect::<Result<_, _>>()?;
    Ok(Value::Object(object([
        ("receiver_id", json!(receiver_id)),
        ("refund_to", json!(refund_to)),
        ("actions", Value::Array(actions)),
    ])))
}

fn action_value(
    action: &NearAction,
    receiver_id: &AccountId,
    token: Option<(u8, &str)>,
    warnings: &mut Vec<String>,
) -> Result<Value, MessageError> {
    Ok(Value::Object(match action {
        NearAction::Transfer(Transfer { amount }) => {
            object([("action", json!("transfer")), ("amount", near(*amount))])
        }
        NearAction::FunctionCall(FunctionCall {
            function_name,
            args,
            deposit,
            gas,
            gas_weight,
        }) => {
            let args = if function_name == EXECUTE_EXTENSION {
                let request = extension_request(args)?;
                let name = format!("wallet {receiver_id}");
                json!({ "request": request_value(&request, receiver_id, &name, warnings)? })
            } else if let Some((_, keys)) = FT_TRANSFER_METHODS
                .iter()
                .find(|(method, _)| method == function_name)
            {
                token_args(args, token.ok_or(MessageError::TokenMetadata)?, keys)?
            } else {
                args_value(args)?
            };
            object([
                ("action", json!("function_call")),
                ("function_name", json!(function_name)),
                ("deposit", near(*deposit)),
                ("gas", json!(gas.as_gas().to_string())),
                ("gas_weight", json!(gas_weight.to_string())),
                ("args", args),
            ])
        }
        NearAction::DeterministicStateInit(DeterministicStateInit {
            state_init,
            deposit,
        }) => object([
            ("action", json!("deterministic_state_init")),
            ("deposit", near(*deposit)),
            ("state_init", sorted(json!(state_init))),
        ]),
        _ => return Err(MessageError::UnsupportedAction),
    }))
}

/// Function arguments are shown as JSON in their original key order.
///
/// They must be exactly the compact serialization of that JSON (no extra
/// whitespace, duplicate keys, optional escapes or alternative number forms),
/// so the message determines the argument bytes. Anything else, including
/// empty and binary arguments, is rejected, as is JSON that JavaScript would
/// serialize differently (see [`portable`]).
fn args_value(args: &[u8]) -> Result<Value, MessageError> {
    let value: Value = serde_json::from_slice(args).map_err(|_| MessageError::FunctionArgs)?;
    if !portable(&value) || serde_json::to_vec(&value)? != args {
        return Err(MessageError::FunctionArgs);
    }
    Ok(value)
}

/// NEP-141 transfer arguments: only the method's `keys`, with the `amount`
/// shown with the token's decimals and symbol. It must be a canonical decimal
/// string, so the shown amount and decimals determine it.
fn token_args(
    args: &[u8],
    (decimals, symbol): (u8, &str),
    keys: &[&str],
) -> Result<Value, MessageError> {
    let mut value = args_value(args)?;
    let object = value.as_object_mut().ok_or(MessageError::TokenAmount)?;
    if object.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err(MessageError::TokenArgs);
    }
    let amount = object.get_mut("amount").ok_or(MessageError::TokenAmount)?;
    let units = amount
        .as_str()
        .and_then(|text| {
            text.parse::<u128>()
                .ok()
                .filter(|units| units.to_string() == text)
        })
        .ok_or(MessageError::TokenAmount)?;
    *amount = json!(token_amount(units, decimals, symbol));
    Ok(value)
}

/// Function arguments that are exactly their compact JSON serialization, also
/// in JavaScript (the rule for arguments shown in readable messages).
pub fn json_args(args: &[u8]) -> Result<Value, MessageError> {
    args_value(args)
}

/// `yoctoNEAR` with 24 decimals and the `NEAR` symbol, e.g. `1.5 NEAR`.
pub fn near_amount(yocto: u128) -> String {
    token_amount(yocto, 24, "NEAR")
}

/// yoctoNEAR attached to a promise (transfer amounts and deposits), saturating
/// at `u128::MAX`: the most that its `refund_to` can receive.
pub fn attached_near(promise: &NearPromise) -> u128 {
    promise
        .actions
        .iter()
        .map(|action| match action {
            NearAction::Transfer(Transfer { amount })
            | NearAction::FunctionCall(FunctionCall {
                deposit: amount, ..
            })
            | NearAction::DeterministicStateInit(DeterministicStateInit {
                deposit: amount, ..
            }) => amount.as_yoctonear(),
            _ => 0,
        })
        .fold(0, u128::saturating_add)
}

/// Largest integer that JavaScript numbers represent exactly (2^53 - 1).
const MAX_SAFE_INTEGER: i64 = (1 << 53) - 1;

/// Whether `JSON.stringify(JSON.parse(..))` gives the same compact bytes as
/// `serde_json`: numbers are integers within ±(2^53 - 1), and objects list
/// array-index keys first, in ascending order, as JavaScript objects do.
/// `serde_json` itself rejects unpaired surrogates.
fn portable(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => true,
        Value::Number(number) => number
            .as_i64()
            .is_some_and(|n| (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&n)),
        Value::Array(items) => items.iter().all(portable),
        Value::Object(object) => {
            object
                .keys()
                .is_sorted_by_key(|key| array_index(key).unwrap_or(u32::MAX))
                && object.values().all(portable)
        }
    }
}

/// JavaScript array index: a canonical decimal below 2^32 - 1.
fn array_index(key: &str) -> Option<u32> {
    key.parse()
        .ok()
        .filter(|index: &u32| *index != u32::MAX && index.to_string() == key)
}

fn near(amount: NearToken) -> Value {
    json!(near_amount(amount.as_yoctonear()))
}

/// `units` with `decimals` fractional digits, without trailing zeros,
/// followed by the symbol, e.g. `1.5 USDC` for 1500000 units with 6 decimals.
fn token_amount(units: u128, decimals: u8, symbol: &str) -> String {
    let decimals = usize::from(decimals);
    let digits = format!("{units:0>width$}", width = decimals + 1);
    let (whole, fraction) = digits.split_at(digits.len() - decimals);
    match fraction.trim_end_matches('0') {
        "" => format!("{whole} {symbol}"),
        fraction => format!("{whole}.{fraction} {symbol}"),
    }
}

/// Message object with fields in the given order.
fn object<const N: usize>(fields: [(&str, Value); N]) -> Map<String, Value> {
    fields
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect()
}

/// State init with keys sorted at every level.
fn sorted(mut value: Value) -> Value {
    value.sort_all_objects();
    value
}

/// Pretty JSON, stopping as soon as it cannot fit the message (escaping only
/// makes it longer), so deeply indented input costs at most the limit.
fn pretty(fields: &Map<String, Value>) -> Result<String, MessageError> {
    struct Capped(Vec<u8>);
    impl std::io::Write for Capped {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0.len() + bytes.len() > MAX_SIGNING_MESSAGE_BYTES {
                return Err(std::io::Error::other("signing message exceeds 16 KiB"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut json = Capped(Vec::new());
    serde_json::to_writer_pretty(&mut json, fields).map_err(|error| {
        if error.is_io() {
            MessageError::TooLarge
        } else {
            error.into()
        }
    })?;
    Ok(String::from_utf8(json.0).expect("JSON is UTF-8"))
}

/// `prefix`, warning lines, then the JSON of `head`, `identity` and `tail` fields.
fn render<const H: usize, const T: usize>(
    prefix: &str,
    warnings: &[String],
    identity: SigningIdentity<'_>,
    head: [(&str, Value); H],
    tail: [(&str, Value); T],
) -> Result<String, MessageError> {
    let mut fields = object(head);
    let identity = identity.iter().map(|&(name, value)| (name, json!(value)));
    for (name, value) in identity.chain(tail) {
        if fields.insert(name.to_owned(), value).is_some() {
            return Err(MessageError::IdentityField);
        }
    }
    let json = pretty(&fields)?;
    let mut message = String::from(prefix);
    // Account IDs in warnings are ASCII.
    for warning in warnings {
        message.push_str(warning);
        message.push('\n');
    }
    if !warnings.is_empty() {
        message.push('\n');
    }
    // Escape DEL and all non-ASCII characters, including bidi controls and line separators.
    // UTF-16 code units make the exact format reproducible in browser JavaScript.
    for unit in json.encode_utf16() {
        if message.len() >= MAX_SIGNING_MESSAGE_BYTES {
            return Err(MessageError::TooLarge);
        }
        if unit <= 0x7e {
            message.push(char::from(u8::try_from(unit).expect("ASCII unit")));
        } else {
            write!(message, "\\u{unit:04x}").expect("String write");
        }
    }
    message.push('\n');
    if message.len() > MAX_SIGNING_MESSAGE_BYTES {
        return Err(MessageError::TooLarge);
    }
    Ok(message)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::Timestamp;

    const IDENTITY: SigningIdentity<'static> = &[("test_public_key", "key")];

    fn request(internal: Vec<WalletOp>, function_name: &str, args: &[u8]) -> RequestMessage {
        RequestMessage {
            pay_for_gas: false,
            chain_id: "mainnet".to_string(),
            signer_id: "alice.near".parse().unwrap(),
            nonce: 1,
            created_at: Timestamp::UNIX_EPOCH,
            timeout: Duration::from_mins(1),
            request: Request {
                internal,
                external: vec![
                    NearPromise::new("token.near".parse::<AccountId>().unwrap())
                        .function_call(FunctionCall::name(function_name).args(args)),
                ],
            },
        }
    }

    fn call(args: &[u8]) -> RequestMessage {
        request(Vec::new(), "submit", args)
    }

    fn ft_transfer(internal: Vec<WalletOp>, amount: &str) -> RequestMessage {
        let args = format!(r#"{{"receiver_id":"intents.near","amount":{amount},"msg":""}}"#);
        request(internal, "ft_transfer_call", args.as_bytes())
    }

    fn usdc(token_id: &str) -> WalletOp {
        WalletOp::require_ft_metadata(token_id.parse::<AccountId>().unwrap(), 6, "USDC")
    }

    fn extension_call(inner: &Request) -> RequestMessage {
        let args = serde_json::to_vec(&json!({ "request": inner })).unwrap();
        request(Vec::new(), EXECUTE_EXTENSION, &args)
    }

    /// Signed JSON document of a rendered request message.
    fn document(message: &str) -> Value {
        let body = message.strip_prefix(REQUEST_PREFIX).unwrap();
        serde_json::from_str(&body[body.find('{').unwrap()..]).unwrap()
    }

    #[test]
    fn fields_have_a_fixed_order_with_operations_first() {
        let mut msg = call(br#"{"z":1}"#);
        msg.request
            .internal
            .push(WalletOp::set_signature_mode(false));
        let message = request_signing_message(IDENTITY, &msg).unwrap();
        let expected = concat!(
            "{\n",
            "  \"chain_id\": \"mainnet\",\n",
            "  \"signer_id\": \"alice.near\",\n",
            "  \"test_public_key\": \"key\",\n",
            "  \"nonce\": 1,\n",
            "  \"created_at\": \"1970-01-01T00:00:00Z\",\n",
            "  \"timeout_secs\": 60,\n",
            "  \"pay_for_gas\": false,\n",
            "  \"request\": {\n",
            "    \"internal\": [\n",
            "      {\n",
            "        \"op\": \"set_signature_mode\",\n",
            "        \"payload\": {\n",
            "          \"enable\": false\n",
            "        }\n",
            "      }\n",
            "    ],\n",
            "    \"external\": [\n",
            "      {\n",
            "        \"receiver_id\": \"token.near\",\n",
            "        \"refund_to\": null,\n",
            "        \"actions\": [\n",
            "          {\n",
            "            \"action\": \"function_call\",\n",
            "            \"function_name\": \"submit\",\n",
            "            \"deposit\": \"0 NEAR\",\n",
            "            \"gas\": \"50000000000000\",\n",
            "            \"gas_weight\": \"1\",\n",
            "            \"args\": {\n",
            "              \"z\": 1\n",
            "            }\n",
            "          }\n",
            "        ]\n",
            "      }\n",
            "    ]\n",
            "  }\n",
            "}\n",
        );
        assert_eq!(message, format!("{REQUEST_PREFIX}{expected}"));
    }

    #[test]
    fn add_extension_is_announced_before_the_message() {
        let mut msg = call(b"{}");
        msg.request.internal = vec![
            WalletOp::add_extension("one.near".parse::<AccountId>().unwrap()),
            WalletOp::add_extension("two.near".parse::<AccountId>().unwrap()),
        ];
        let message = request_signing_message(IDENTITY, &msg).unwrap();
        assert!(message.starts_with(&format!(
            "{REQUEST_PREFIX}{}{}\n{{\n",
            "WARNING: add_extension gives one.near full control of this wallet.\n",
            "WARNING: add_extension gives two.near full control of this wallet.\n",
        )));
        assert_eq!(
            document(&message)["request"]["internal"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn extension_requests_are_shown_by_the_same_rules() {
        let inner = Request {
            internal: vec![
                usdc("usdc.near"),
                WalletOp::add_extension("evil.near".parse::<AccountId>().unwrap()),
            ],
            external: vec![
                NearPromise::new("usdc.near".parse::<AccountId>().unwrap()).function_call(
                    FunctionCall::name("ft_transfer")
                        .args(br#"{"receiver_id":"bob.near","amount":"1500000"}"#.to_vec()),
                ),
            ],
        };
        let message = request_signing_message(IDENTITY, &extension_call(&inner)).unwrap();
        assert!(message.contains(
            "\nWARNING: add_extension gives evil.near full control of wallet token.near.\n"
        ));
        let shown = &document(&message)["request"]["external"][0]["actions"][0]["args"];
        assert_eq!(
            shown["request"]["internal"][1]["payload"]["account_id"],
            "evil.near"
        );
        assert_eq!(
            shown["request"]["external"][0]["actions"][0]["args"],
            json!({"receiver_id": "bob.near", "amount": "1.5 USDC"})
        );
        assert!(!message.contains("eyJ"));

        // Inner NEP-141 transfers need their own metadata requirement.
        let unchecked = Request {
            internal: Vec::new(),
            ..inner.clone()
        };
        assert!(matches!(
            request_signing_message(IDENTITY, &extension_call(&unchecked)),
            Err(MessageError::TokenMetadata)
        ));
        // Unknown fields, other arguments and any encoding other than the
        // serde form (which the message determines) are rejected.
        let transfer = json!({"action": "transfer", "payload": {"amount": "1"}});
        for args in [
            json!({"request": inner, "note": "ignored"}),
            json!({"request": {"internal": [], "max_spend": "1"}}),
            json!({}),
            json!({"request": {"internal": [], "external": []}}),
            json!({"request": {"external": [{"receiver_id": "a.near", "refund_to": null, "actions": [transfer]}]}}),
            json!({"request": {"external": [{"actions": [transfer], "receiver_id": "a.near"}]}}),
            json!({"request": {"external": [{"receiver_id": "a.near", "actions": [{"action": "transfer", "payload": {"amount": "01"}}]}]}}),
            json!({"request": {"external": [{"receiver_id": "a.near", "actions": [{"action": "function_call", "payload": {"function_name": "f", "args": "e30=", "gas_weight": "1"}}]}]}}),
        ] {
            let msg = request(
                Vec::new(),
                EXECUTE_EXTENSION,
                &serde_json::to_vec(&args).unwrap(),
            );
            assert!(matches!(
                request_signing_message(IDENTITY, &msg),
                Err(MessageError::FunctionArgs)
            ));
        }
    }

    #[test]
    fn refunds_to_another_account_are_announced() {
        let promise = |refund_to: Option<&str>, yocto: u128| {
            let promise = NearPromise::new("bob.near".parse::<AccountId>().unwrap())
                .transfer(NearToken::from_yoctonear(yocto))
                .function_call(
                    FunctionCall::name("f")
                        .args(b"{}".to_vec())
                        .attach_deposit(NearToken::from_yoctonear(yocto)),
                );
            match refund_to {
                Some(account) => promise.refund_to(account.parse::<AccountId>().unwrap()),
                None => promise,
            }
        };
        let with = |request: Request| RequestMessage {
            request,
            ..call(b"{}")
        };
        // Warning lines between the prefix and the JSON.
        let warnings = |msg: &RequestMessage| {
            let message = request_signing_message(IDENTITY, msg).unwrap();
            message[REQUEST_PREFIX.len()..message.find("{\n").unwrap()].to_owned()
        };
        assert_eq!(
            warnings(&with(
                promise(Some("carol.near"), 15 * 10_u128.pow(23)).into()
            )),
            "WARNING: up to 3 NEAR sent to bob.near can be refunded to carol.near, not to this wallet.\n\n"
        );
        // Refunds to the executing wallet, or with nothing attached, need none.
        for request in [
            promise(None, 1),
            promise(Some("alice.near"), 1),
            promise(Some("carol.near"), 0),
        ] {
            assert_eq!(warnings(&with(request.into())), "");
        }
        // A nested request is executed by the wallet it is sent to.
        assert_eq!(
            warnings(&extension_call(&promise(Some("carol.near"), 1).into())),
            "WARNING: up to 0.000000000000000000000002 NEAR sent to bob.near can be refunded to carol.near, not to wallet token.near.\n\n"
        );
        assert_eq!(
            warnings(&extension_call(&promise(Some("token.near"), 1).into())),
            ""
        );
    }

    #[test]
    fn token_transfers_accept_only_the_method_parameters() {
        let transfer = |method: &str, args: &str| {
            request_signing_message(
                IDENTITY,
                &request(vec![usdc("token.near")], method, args.as_bytes()),
            )
        };
        transfer(
            "ft_transfer",
            r#"{"receiver_id":"bob.near","amount":"1","memo":"m"}"#,
        )
        .unwrap();
        transfer(
            "ft_transfer_call",
            r#"{"receiver_id":"bob.near","amount":"1","memo":null,"msg":""}"#,
        )
        .unwrap();
        // Keys the token ignores could imitate a shown amount.
        for (method, args) in [
            (
                "ft_transfer",
                r#"{"receiver_id":"bob.near"," amount":"5 USDC","amount":"5000000000"}"#,
            ),
            (
                "ft_transfer",
                r#"{"receiver_id":"bob.near","amount":"1","msg":""}"#,
            ),
            (
                "ft_transfer_call",
                r#"{"receiver_id":"bob.near","amount":"1","msg":"","amount_display":"1 USDC"}"#,
            ),
        ] {
            assert!(
                matches!(transfer(method, args), Err(MessageError::TokenArgs)),
                "accepted {args}"
            );
        }
    }

    #[test]
    fn nested_state_init_keys_need_the_same_order_in_serde_and_javascript() {
        // The arguments must be portable JSON (array-index keys first) and the
        // serde form (keys in byte order). The byte order of "1000" and "2000"
        // agrees with JavaScript; "AAAA" decodes to zeros, so it sorts first.
        let nested = |data: &str| {
            let args = format!(
                r#"{{"request":{{"external":[{{"receiver_id":"a.near","actions":[{{"action":"deterministic_state_init","payload":{{"state_init":{{"V1":{{"code":{{"account_id":"code.near"}},"data":{data}}}}}}}}}]}}]}}}}"#
            );
            request_signing_message(
                IDENTITY,
                &request(Vec::new(), EXECUTE_EXTENSION, args.as_bytes()),
            )
        };
        nested(r#"{"1000":"","2000":""}"#).unwrap();
        for data in [r#"{"1000":"","AAAA":""}"#, r#"{"AAAA":"","1000":""}"#] {
            assert!(
                matches!(nested(data), Err(MessageError::FunctionArgs)),
                "accepted {data}"
            );
        }
    }

    #[test]
    fn oversized_inputs_are_rejected_before_parsing() {
        // Invalid JSON: rejected as too large, without being parsed.
        let args = vec![b'x'; MAX_SIGNING_MESSAGE_BYTES + 1];
        assert!(matches!(
            request_signing_message(IDENTITY, &call(&args)),
            Err(MessageError::TooLarge)
        ));
        // Too many operations for the limit, whatever their content.
        let mut msg = call(b"{}");
        msg.request.internal = vec![WalletOp::set_signature_mode(true); 256];
        assert!(matches!(
            request_signing_message(IDENTITY, &msg),
            Err(MessageError::TooLarge)
        ));
        // A token symbol (shown in each transfer amount) counts, too.
        let mut msg = call(b"x");
        msg.request.internal = vec![WalletOp::require_ft_metadata(
            "token.near".parse::<AccountId>().unwrap(),
            6,
            "S".repeat(MAX_SIGNING_MESSAGE_BYTES),
        )];
        assert!(matches!(
            request_signing_message(IDENTITY, &msg),
            Err(MessageError::TooLarge)
        ));
        // Short but deeply nested arguments stop growing at the limit.
        let deep = format!("{}{}", "[".repeat(120), "]".repeat(120));
        assert!(matches!(
            request_signing_message(IDENTITY, &call(deep.as_bytes())),
            Err(MessageError::TooLarge)
        ));
        let auth = OffchainMessage {
            chain_id: "mainnet".to_string(),
            signer_id: "alice.near".parse().unwrap(),
            path: Vec::new(),
            timestamp: Timestamp::UNIX_EPOCH,
            payload: "x".repeat(MAX_SIGNING_MESSAGE_BYTES + 1),
        };
        assert!(matches!(
            offchain_signing_message(IDENTITY, &auth),
            Err(MessageError::TooLarge)
        ));
        // Path accounts count, too.
        let auth = OffchainMessage {
            path: vec!["a".repeat(64).parse().unwrap(); 300],
            payload: "x".to_string(),
            ..auth
        };
        assert!(matches!(
            offchain_signing_message(IDENTITY, &auth),
            Err(MessageError::TooLarge)
        ));
    }

    #[test]
    fn args_are_shown_once_in_original_order_and_determine_the_bytes() {
        let args = r#"{"receiver_id":"intents.near","amount":"1500000","msg":"{\"memo\":\"Оплата\"}","items":[{"z":null,"a":true}]}"#;
        let message = request_signing_message(IDENTITY, &call(args.as_bytes())).unwrap();
        assert!(message.contains(concat!(
            "            \"args\": {\n",
            "              \"receiver_id\": \"intents.near\",\n",
            "              \"amount\": \"1500000\",\n",
            "              \"msg\": \"{\\\"memo\\\":\\\"\\u041e\\u043f\\u043b\\u0430\\u0442\\u0430\\\"}\",\n",
            "              \"items\": [\n",
            "                {\n",
            "                  \"z\": null,\n",
            "                  \"a\": true\n",
            "                }\n",
            "              ]\n",
            "            }\n",
        )));
        assert!(!message.contains("args_"));
        let shown = &document(&message)["request"]["external"][0]["actions"][0]["args"];
        assert_eq!(serde_json::to_string(shown).unwrap(), args);
    }

    #[test]
    fn reordered_args_change_the_message() {
        assert_ne!(
            request_signing_message(IDENTITY, &call(br#"{"a":1,"b":2}"#)).unwrap(),
            request_signing_message(IDENTITY, &call(br#"{"b":2,"a":1}"#)).unwrap(),
        );
    }

    #[test]
    fn args_other_than_compact_json_are_rejected() {
        for args in [
            &b""[..],
            b"\x00\xff",
            b"\xef\xbb\xbf{}",
            b"not json",
            b" {}",
            b"{}\n",
            br#"{"a": 1}"#,
            br#"{"a":1,"a":2}"#,
            br#"{"a":"\u0041"}"#,
            br#"{"a":"\/"}"#,
            br#"{"a":1e3}"#,
            br#"{"a":1.50}"#,
            br#"{"a":-0}"#,
            br#"{"a":18446744073709551616}"#,
        ] {
            assert!(
                matches!(
                    request_signing_message(IDENTITY, &call(args)),
                    Err(MessageError::FunctionArgs)
                ),
                "accepted {:?}",
                String::from_utf8_lossy(args)
            );
        }
        for args in [
            &b"{}"[..],
            b"[]",
            b"null",
            b"\"text\"",
            br#"{"a":-1,"b":true}"#,
        ] {
            request_signing_message(IDENTITY, &call(args)).unwrap();
        }
    }

    #[test]
    fn args_have_the_same_compact_form_in_javascript() {
        // `JSON.stringify(JSON.parse(args)) == args` in JavaScript, too.
        for args in [
            &br#"{"a":-9007199254740991,"b":9007199254740991}"#[..],
            br#"{"0":1,"2":2,"10":3,"b":4,"4294967295":5,"a":6}"#,
            br#"[{"1":null},"x"]"#,
        ] {
            request_signing_message(IDENTITY, &call(args)).unwrap();
        }
        for args in [
            &br#"{"a":0.000001}"#[..],
            br#"{"a":0.5}"#,
            br#"{"a":1.0}"#,
            br#"{"a":100000000000000000000}"#,
            br#"{"a":9007199254740992}"#,
            br#"{"a":-9007199254740992}"#,
            br#"{"a":9007199254740993}"#,
            br#"{"a":"\ud800"}"#,
            br#"{"2":1,"1":2}"#,
            br#"{"a":1,"1":2}"#,
            br#"{"4294967295":1,"1":2}"#,
            br#"[{"b":1,"0":2}]"#,
        ] {
            assert!(
                matches!(
                    request_signing_message(IDENTITY, &call(args)),
                    Err(MessageError::FunctionArgs)
                ),
                "accepted {:?}",
                String::from_utf8_lossy(args)
            );
        }
    }

    #[test]
    fn amounts_have_decimals_and_symbol() {
        for (units, decimals, shown) in [
            (0, 24, "0 NEAR"),
            (1, 24, "0.000000000000000000000001 NEAR"),
            (10_u128.pow(24), 24, "1 NEAR"),
            (15 * 10_u128.pow(23), 24, "1.5 NEAR"),
            (
                u128::MAX,
                24,
                "340282366920938.463463374607431768211455 NEAR",
            ),
            (1_500_000, 6, "1.5 NEAR"),
            (1_500_000, 0, "1500000 NEAR"),
        ] {
            assert_eq!(token_amount(units, decimals, "NEAR"), shown);
        }
        let message = request_signing_message(
            IDENTITY,
            &RequestMessage {
                request: NearPromise::new("bob.near".parse::<AccountId>().unwrap())
                    .transfer(NearToken::from_millinear(1500))
                    .into(),
                ..call(b"{}")
            },
        )
        .unwrap();
        assert_eq!(
            document(&message)["request"]["external"][0]["actions"][0],
            json!({"action": "transfer", "amount": "1.5 NEAR"})
        );
        assert!(!message.contains("yocto"));
    }

    #[test]
    fn token_amounts_use_the_required_metadata() {
        let message = request_signing_message(
            IDENTITY,
            &ft_transfer(vec![usdc("token.near")], r#""1500000""#),
        )
        .unwrap();
        let document = document(&message);
        assert_eq!(
            document["request"]["internal"][0],
            json!({
                "op": "require_ft_metadata",
                "payload": {"token_id": "token.near", "decimals": 6, "symbol": "USDC"},
            })
        );
        assert_eq!(
            document["request"]["external"][0]["actions"][0],
            json!({
                "action": "function_call",
                "function_name": "ft_transfer_call",
                "args": {"receiver_id": "intents.near", "amount": "1.5 USDC", "msg": ""},
                "deposit": "0 NEAR",
                "gas": "50000000000000",
                "gas_weight": "1",
            })
        );
    }

    #[test]
    fn token_transfers_need_metadata_and_a_canonical_amount() {
        for internal in [
            Vec::new(),
            vec![usdc("other.near")],
            vec![usdc("token.near"), usdc("token.near")],
        ] {
            assert!(matches!(
                request_signing_message(IDENTITY, &ft_transfer(internal, r#""1""#)),
                Err(MessageError::TokenMetadata)
            ));
        }
        for amount in [
            r#""01""#,
            r#""+1""#,
            r#""1.5""#,
            r#""""#,
            "1",
            "null",
            r#""340282366920938463463374607431768211456""#,
        ] {
            assert!(
                matches!(
                    request_signing_message(
                        IDENTITY,
                        &ft_transfer(vec![usdc("token.near")], amount)
                    ),
                    Err(MessageError::TokenAmount)
                ),
                "accepted {amount}"
            );
        }
        assert!(matches!(
            request_signing_message(
                IDENTITY,
                &request(
                    vec![usdc("token.near")],
                    "ft_transfer",
                    br#"{"receiver_id":"bob.near"}"#
                )
            ),
            Err(MessageError::TokenAmount)
        ));
    }

    #[test]
    fn identity_cannot_replace_message_fields() {
        for name in ["nonce", "chain_id", "request"] {
            assert!(matches!(
                request_signing_message(&[(name, "2")], &call(b"{}")),
                Err(MessageError::IdentityField)
            ));
        }
    }
}
