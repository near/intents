#![doc = include_str!("../README.md")]

use std::{
    borrow::Borrow,
    io::{self, ErrorKind},
    mem,
};

use defuse_wallet::{
    AccountId, NearPromise, NearToken, Request, RequestMessage, StateInit, StateInitV1, Timestamp,
    WalletOp,
    actions::{DeterministicStateInit, FunctionCall, NearAction, Transfer},
    offchain::OffchainMessage,
};
use serde::{Serialize, Serializer, ser};
use serde_json::{Value, json, ser::Formatter};

/// Maximum UTF-8 message size accepted by this schema (16 KiB).
pub const MAX_SIGNING_MESSAGE_BYTES: usize = 16 * 1024;
/// Domain and purpose separator for execution requests.
pub const REQUEST_PREFIX: &str = r"NEAR Wallet Contract: execute request (v1)
This signature authorizes NEAR wallet operations, not a login.
NEAR amounts and deposits are in yoctoNEAR; FT amounts are in token base units.
Gas is in gas units.
Function arguments and deployment state may be opaque: verify them before signing.

";
/// Domain and purpose separator for NEP-641 authorizations.
pub const OFFCHAIN_PREFIX: &str = r"NEAR Wallet Contract: offchain authorization (v1)
This signature authorizes the exact payload below, not a NEAR transaction.
The consuming application defines expiration and one-time-use requirements.

";

/// A message cannot be represented by this schema.
#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("unsupported action")]
    UnsupportedAction,
    #[error("function call arguments must be compact JSON")]
    FunctionArgs,
    #[error("NEP-141 transfer amount must be a decimal u128 string")]
    TokenAmount,
    #[error("NEP-141 transfer arguments must be parameters of the method")]
    TokenArgs,
    #[error("NEP-141 transfers require exactly one yoctoNEAR")]
    TokenDeposit,
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
    let input_bytes = input_bytes(request);

    if input_bytes > MAX_SIGNING_MESSAGE_BYTES {
        return Err(MessageError::TooLarge);
    }

    let mut warnings = Vec::new();
    let request = Shown::new(request, signer_id, "this wallet", &mut warnings)?;

    render(
        REQUEST_PREFIX,
        &warnings,
        input_bytes,
        identity,
        [
            ("chain_id", Field::Text(chain_id)),
            ("signer_id", Field::Text(signer_id.as_str())),
        ],
        [
            ("nonce", Field::Number(*nonce)),
            ("created_at", Field::Time(created_at)),
            ("timeout_secs", Field::Number(timeout_secs)),
            ("pay_for_gas", Field::Flag(*pay_for_gas)),
            ("request", Field::Request(&request)),
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
        shown,
        identity,
        [
            ("chain_id", Field::Text(chain_id)),
            ("signer_id", Field::Text(signer_id.as_str())),
        ],
        [
            ("path", Field::Path(path)),
            ("timestamp", Field::Time(timestamp)),
            ("payload", Field::Text(payload)),
        ],
    )
}

/// Each operation, promise and action renders to more bytes than this.
const MIN_ITEM_BYTES: usize = 64;

/// The v1 input budget includes raw arguments, state data and a charge per item.
/// Nested arguments contain base64 that may render shorter. Bound method names
/// separately since adding them to that budget would reject valid v1 messages.
fn input_bytes(request: &Request) -> usize {
    let items = request.internal.len() + request.external.len();
    let mut input = items.saturating_mul(MIN_ITEM_BYTES);
    let mut names = 0usize;

    for action in request.external.iter().flat_map(|promise| &promise.actions) {
        let data_bytes = match action {
            NearAction::FunctionCall(FunctionCall {
                function_name,
                args,
                ..
            }) => {
                names = names.saturating_add(function_name.len());
                args.len()
            }
            NearAction::DeterministicStateInit(DeterministicStateInit {
                state_init: StateInit::V1(StateInitV1 { data, .. }),
                ..
            }) => data
                .iter()
                .map(|(key, value)| key.len() + value.len())
                .sum(),
            _ => 0,
        };

        input = input
            .saturating_add(MIN_ITEM_BYTES)
            .saturating_add(data_bytes);
    }

    input.max(names)
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

/// The request in `w_execute_extension()` arguments, which must be portable
/// JSON and match the compact serde form of `ExtensionArgs` byte for byte.
fn extension_request(args: &[u8]) -> Result<Request, MessageError> {
    let extension: ExtensionArgs =
        serde_json::from_value(portable_args(args)?).map_err(|_| MessageError::FunctionArgs)?;

    if !is_compact_json(&extension, args)? {
        return Err(MessageError::FunctionArgs);
    }

    Ok(extension.request)
}

/// Whether the compact JSON of `value` is exactly `bytes`.
fn is_compact_json(value: &impl Serialize, bytes: &[u8]) -> Result<bool, MessageError> {
    // Equal JSON has this size, so it is allocated once.
    let mut json = Vec::with_capacity(bytes.len());

    serde_json::to_writer(&mut json, value)?;
    Ok(json == bytes)
}

/// A request and what its actions show in place of their bytes. `R` borrows
/// the signed request, or owns one parsed from `w_execute_extension()`
/// arguments.
///
/// It is serialized straight into the message, without an intermediate JSON
/// tree: only function arguments and state init are kept as [`Value`]s.
struct Shown<R> {
    request: R,
    /// For each promise, for each of its actions.
    actions: Vec<Vec<Parsed>>,
}

/// What an action shows in place of its bytes.
#[derive(Serialize)]
#[serde(untagged)]
enum Parsed {
    /// A transfer has nothing but its amount.
    Nothing,
    /// Function arguments, or state init with sorted keys.
    Json(Value),
    /// `w_execute_extension()` arguments.
    Extension { request: Shown<Request> },
}

impl<R: Borrow<Request>> Shown<R> {
    /// Checks the request and collects its warnings: wallet operations first,
    /// then external promises. `wallet` is the account that executes
    /// `request`, and `name` refers to it in warnings.
    fn new(
        request: R,
        wallet: &AccountId,
        name: &str,
        warnings: &mut Vec<String>,
    ) -> Result<Self, MessageError> {
        let Request { internal, external } = request.borrow();

        for op in internal {
            if let WalletOp::AddExtension { account_id } = op {
                warnings.push(format!(
                    "WARNING: add_extension gives {account_id} full control of {name}."
                ));
            }
        }

        let actions = external
            .iter()
            .map(|promise| parse_promise(promise, (wallet, name), warnings))
            .collect::<Result<_, _>>()?;

        Ok(Self { request, actions })
    }
}

impl<R: Borrow<Request>> Serialize for Shown<R> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct RequestView<'a, P> {
            internal: &'a [WalletOp],
            external: P,
        }

        #[derive(Serialize)]
        struct PromiseView<'a, A> {
            receiver_id: &'a AccountId,
            refund_to: &'a Option<AccountId>,
            actions: A,
        }

        let Request { internal, external } = self.request.borrow();
        let promises = external.iter().zip(&self.actions).map(|(promise, parsed)| {
            let NearPromise {
                receiver_id,
                refund_to,
                actions,
            } = promise;
            let actions = actions
                .iter()
                .zip(parsed)
                .map(|(action, parsed)| ShownAction { action, parsed });

            PromiseView {
                receiver_id,
                refund_to,
                actions: Sequence(actions),
            }
        });

        RequestView {
            internal,
            external: Sequence(promises),
        }
        .serialize(serializer)
    }
}

/// Items of an iterator as a JSON array.
struct Sequence<I>(I);

impl<I: Iterator<Item: Serialize> + Clone> Serialize for Sequence<I> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.clone())
    }
}

/// NEP-141 methods and their parameters: other arguments, which the token would
/// ignore, could imitate a shown amount and are rejected.
const FT_TRANSFER_METHODS: [(&str, &[&str]); 2] = [
    ("ft_transfer", &["receiver_id", "amount", "memo"]),
    (
        "ft_transfer_call",
        &["receiver_id", "amount", "memo", "msg"],
    ),
];

/// `wallet` executes the promise; `refund_to` (`null` means `wallet`) receives
/// its failed and unused deposits, e.g. the unused part of a state-init budget.
fn parse_promise(
    promise: &NearPromise,
    (wallet, name): (&AccountId, &str),
    warnings: &mut Vec<String>,
) -> Result<Vec<Parsed>, MessageError> {
    let NearPromise {
        receiver_id,
        refund_to,
        actions,
    } = promise;
    if let Some(refund_to) = refund_to.as_ref().filter(|account| *account != wallet) {
        // Transfer amounts and deposits: the most that `refund_to` can receive.
        let attached = promise.total_deposit();

        if !attached.is_zero() {
            warnings.push(format!(
                "WARNING: up to {} sent to {receiver_id} can be refunded to {refund_to}, not to {name}.",
                near_amount(attached.as_yoctonear())
            ));
        }
    }

    actions
        .iter()
        .map(|action| parse_action(action, receiver_id, warnings))
        .collect()
}

fn parse_action(
    action: &NearAction,
    receiver_id: &AccountId,
    warnings: &mut Vec<String>,
) -> Result<Parsed, MessageError> {
    Ok(match action {
        NearAction::Transfer(_) => Parsed::Nothing,
        NearAction::FunctionCall(FunctionCall {
            function_name,
            args,
            deposit,
            ..
        }) => {
            if function_name == EXECUTE_EXTENSION {
                let request = extension_request(args)?;
                let name = format!("wallet {receiver_id}");

                Parsed::Extension {
                    request: Shown::new(request, receiver_id, &name, warnings)?,
                }
            } else if let Some((_, keys)) = FT_TRANSFER_METHODS
                .iter()
                .find(|(method, _)| method == function_name)
            {
                // Reject a predictably failing token call before other wallet effects commit.
                if deposit.as_yoctonear() != 1 {
                    return Err(MessageError::TokenDeposit);
                }

                Parsed::Json(token_args(args, keys)?)
            } else {
                Parsed::Json(args_value(args)?)
            }
        }
        NearAction::DeterministicStateInit(DeterministicStateInit { state_init, .. }) => {
            Parsed::Json(sorted(json!(state_init)))
        }
        _ => return Err(MessageError::UnsupportedAction),
    })
}

/// An action with `parsed` in place of its arguments or state.
struct ShownAction<'a> {
    action: &'a NearAction,
    parsed: &'a Parsed,
}

impl Serialize for ShownAction<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(tag = "action", rename_all = "snake_case")]
        enum ActionView<'a> {
            Transfer {
                amount: &'a NearToken,
            },
            FunctionCall {
                function_name: &'a str,
                deposit: &'a NearToken,
                gas: Decimal,
                gas_weight: Decimal,
                args: &'a Parsed,
            },
            DeterministicStateInit {
                deposit: &'a NearToken,
                state_init: &'a Parsed,
            },
        }

        let parsed = self.parsed;

        // No `..`, as in `request_signing_message()`.
        match self.action {
            NearAction::Transfer(Transfer { amount }) => ActionView::Transfer { amount },
            NearAction::FunctionCall(FunctionCall {
                function_name,
                args: _,
                deposit,
                gas,
                gas_weight,
            }) => ActionView::FunctionCall {
                function_name,
                deposit,
                gas: Decimal(gas.as_gas()),
                gas_weight: Decimal(*gas_weight),
                args: parsed,
            },
            NearAction::DeterministicStateInit(DeterministicStateInit {
                state_init: _,
                deposit,
            }) => ActionView::DeterministicStateInit {
                deposit,
                state_init: parsed,
            },
            // `parse_action()` has already rejected it.
            _ => return Err(ser::Error::custom(MessageError::UnsupportedAction)),
        }
        .serialize(serializer)
    }
}

/// Integer shown as a decimal string.
struct Decimal(u64);

impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0)
    }
}

/// Function arguments are shown as JSON in their original key order.
///
/// They must be exactly the compact serialization of that JSON (no extra
/// whitespace, duplicate keys, optional escapes or alternative number forms),
/// so the message determines the argument bytes. Anything else, including
/// empty and binary arguments, is rejected, as is JSON that JavaScript would
/// serialize differently (see [`portable`]).
fn args_value(args: &[u8]) -> Result<Value, MessageError> {
    let value = portable_args(args)?;

    if !is_compact_json(&value, args)? {
        return Err(MessageError::FunctionArgs);
    }

    Ok(value)
}

/// Arguments parsed as JSON that JavaScript serializes in the same way (see
/// [`portable`]).
fn portable_args(args: &[u8]) -> Result<Value, MessageError> {
    let value = serde_json::from_slice(args).map_err(|_| MessageError::FunctionArgs)?;

    if !portable(&value) {
        return Err(MessageError::FunctionArgs);
    }

    Ok(value)
}

/// NEP-141 transfer arguments: only the method's `keys`, with the `amount`
/// left as an exact base-unit integer string. The promise receiver identifies
/// the token.
fn token_args(args: &[u8], keys: &[&str]) -> Result<Value, MessageError> {
    let value = args_value(args)?;
    let object = value.as_object().ok_or(MessageError::TokenArgs)?;

    if object.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err(MessageError::TokenArgs);
    }

    // A key whitelist alone permits incomplete calls such as {"amount":"1"}.
    if object
        .get("receiver_id")
        .and_then(Value::as_str)
        .is_none_or(|id| AccountId::validate(id).is_err())
        || (keys.contains(&"msg") && !object.get("msg").is_some_and(Value::is_string))
        || object
            .get("memo")
            .is_some_and(|memo| !memo.is_null() && !memo.is_string())
    {
        return Err(MessageError::TokenArgs);
    }

    let amount = object.get("amount").ok_or(MessageError::TokenAmount)?;

    if !amount.as_str().is_some_and(canonical_u128) {
        return Err(MessageError::TokenAmount);
    }

    Ok(value)
}

/// Decimal `u128` as Rust prints it: no sign and no leading zeros.
fn canonical_u128(text: &str) -> bool {
    matches!(text.as_bytes(), [b'0'] | [b'1'..=b'9', ..]) && text.parse::<u128>().is_ok()
}

/// Exact smallest-unit integer with an explicit unit for refund warnings.
fn near_amount(yocto: u128) -> String {
    format!("{yocto} yoctoNEAR")
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
    match key.as_bytes() {
        [b'0'] => Some(0),
        [b'1'..=b'9', ..] => key.parse::<u32>().ok().filter(|&index| index != u32::MAX),
        _ => None,
    }
}

/// State init with the keys of every object in the order of a JavaScript
/// object built from them: array-index keys first, in ascending order, then
/// the other keys sorted as strings. `JSON.stringify` gives the same order.
fn sorted(mut value: Value) -> Value {
    sort(&mut value);
    value
}

fn sort(value: &mut Value) {
    // `array_index` is never `u32::MAX`, so the other keys come last.
    fn order(key: &String) -> (u32, &String) {
        (array_index(key).unwrap_or(u32::MAX), key)
    }

    match value {
        Value::Array(items) => items.iter_mut().for_each(sort),
        Value::Object(object) => {
            object.values_mut().for_each(sort);

            // Serde already gives this order, unless a key is an array index.
            if !object.keys().is_sorted_by_key(order) {
                let mut entries: Vec<_> = mem::take(object).into_iter().collect();

                entries.sort_unstable_by(|(a, _), (b, _)| order(a).cmp(&order(b)));
                *object = entries.into_iter().collect();
            }
        }
        _ => {}
    }
}

/// JSON of the message: two spaces of indentation and `": "` after keys, in
/// ASCII. DEL and all non-ASCII characters, including bidi controls and line
/// separators, are escaped as lowercase `\uXXXX` UTF-16 code units, which makes
/// the exact format reproducible in browser JavaScript.
#[derive(Default)]
struct AsciiPretty {
    /// Number of open arrays and objects.
    depth: usize,
    /// Whether the innermost of them has a value, so it closes on a new line.
    has_value: bool,
}

impl AsciiPretty {
    /// A comma after a previous value, then a new indented line.
    fn new_line<W: ?Sized + io::Write>(&self, writer: &mut W, comma: bool) -> io::Result<()> {
        const SPACES: &[u8] = &[b' '; 64];

        writer.write_all(if comma { b",\n" } else { b"\n" })?;

        // One write, unless function arguments are deeply nested.
        let mut indent = 2 * self.depth;

        while indent > 0 {
            let spaces = &SPACES[..indent.min(SPACES.len())];

            writer.write_all(spaces)?;
            indent -= spaces.len();
        }

        Ok(())
    }

    fn open<W: ?Sized + io::Write>(&mut self, writer: &mut W, bracket: &[u8]) -> io::Result<()> {
        self.depth += 1;
        self.has_value = false;
        writer.write_all(bracket)
    }

    fn close<W: ?Sized + io::Write>(&mut self, writer: &mut W, bracket: &[u8]) -> io::Result<()> {
        self.depth -= 1;

        if self.has_value {
            self.new_line(writer, false)?;
        }

        writer.write_all(bracket)
    }
}

impl Formatter for AsciiPretty {
    /// The part of a string between the characters that JSON itself escapes.
    fn write_string_fragment<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        const HEX: &[u8; 16] = b"0123456789abcdef";

        let mut rest = fragment;

        // A byte above `~` is DEL or starts a non-ASCII character: the bytes
        // that continue a character follow its first byte.
        while let Some(at) = rest.bytes().position(|byte| byte > b'~') {
            let (ascii, tail) = rest.split_at(at);
            let mut chars = tail.chars();
            let Some(char) = chars.next() else { break };

            writer.write_all(ascii.as_bytes())?;

            for unit in char.encode_utf16(&mut [0; 2]) {
                let hex = |shift: u8| HEX[usize::from(*unit >> shift & 0xf)];

                writer.write_all(&[b'\\', b'u', hex(12), hex(8), hex(4), hex(0)])?;
            }

            rest = chars.as_str();
        }

        writer.write_all(rest.as_bytes())
    }

    fn begin_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.open(writer, b"[")
    }

    fn end_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.close(writer, b"]")
    }

    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.new_line(writer, !first)
    }

    fn end_array_value<W: ?Sized + io::Write>(&mut self, _writer: &mut W) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }

    fn begin_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.open(writer, b"{")
    }

    fn end_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.close(writer, b"}")
    }

    fn begin_object_key<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.new_line(writer, !first)
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b": ")
    }

    fn end_object_value<W: ?Sized + io::Write>(&mut self, _writer: &mut W) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }
}

/// Value of a top-level message field.
#[derive(Clone, Copy, Serialize)]
#[serde(untagged)]
enum Field<'a> {
    Text(&'a str),
    Number(u32),
    Flag(bool),
    Time(&'a Timestamp),
    Path(&'a [AccountId]),
    Request(&'a Shown<&'a Request>),
}

/// Writer of the JSON into the message, up to the size that it has room for.
struct Capped<'a> {
    message: &'a mut Vec<u8>,
    room: usize,
}

impl io::Write for Capped<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.write_all(bytes)?;
        Ok(bytes.len())
    }

    // The JSON is written in many small pieces: no loop for each of them.
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.room = self
            .room
            .checked_sub(bytes.len())
            .ok_or(ErrorKind::WriteZero)?;
        self.message.extend_from_slice(bytes);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Usual size of the JSON for the size checked before rendering: indentation,
/// field names and base64 make it two to four times longer, on top of the
/// fixed fields. It only sizes the buffer of a message, which grows if needed.
const fn usual_json_bytes(input_bytes: usize) -> usize {
    input_bytes.saturating_mul(4).saturating_add(512)
}

/// `prefix`, warning lines, then the JSON of `head`, `identity` and `tail`
/// fields. `input_bytes` is the size checked before rendering.
fn render<const H: usize, const T: usize>(
    prefix: &str,
    warnings: &[String],
    input_bytes: usize,
    identity: SigningIdentity<'_>,
    head: [(&str, Field<'_>); H],
    tail: [(&str, Field<'_>); T],
) -> Result<String, MessageError> {
    let identity = identity
        .iter()
        .map(|&(name, value)| (name, Field::Text(value)));
    let fields = head.into_iter().chain(identity).chain(tail);
    let mut names = fields.clone().map(|(name, _)| name);

    while let Some(name) = names.next() {
        if names.clone().any(|other| other == name) {
            return Err(MessageError::IdentityField);
        }
    }

    // Reserve the final LF and the blank line after warnings before copying a
    // header that might already exceed the whole message budget.
    let framing = prefix.len() + 1 + usize::from(!warnings.is_empty());
    let fixed_bytes = warnings.iter().fold(framing, |bytes, warning| {
        bytes.saturating_add(warning.len()).saturating_add(1)
    });
    let room = MAX_SIGNING_MESSAGE_BYTES
        .checked_sub(fixed_bytes)
        .ok_or(MessageError::TooLarge)?;
    let mut message = Vec::with_capacity(fixed_bytes + usual_json_bytes(input_bytes).min(room));

    message.extend_from_slice(prefix.as_bytes());

    // Account IDs in warnings are ASCII.
    for warning in warnings {
        message.extend_from_slice(warning.as_bytes());
        message.push(b'\n');
    }

    if !warnings.is_empty() {
        message.push(b'\n');
    }

    // The JSON stops as soon as it cannot fit the message with its final LF,
    // so deeply indented input costs at most the limit.
    let json = Capped {
        message: &mut message,
        room,
    };

    serde_json::Serializer::with_formatter(json, AsciiPretty::default())
        .collect_map(fields)
        .map_err(|error| {
            if error.io_error_kind() == Some(ErrorKind::WriteZero) {
                MessageError::TooLarge
            } else {
                error.into()
            }
        })?;
    message.push(b'\n');

    Ok(String::from_utf8(message).expect("message is ASCII"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use defuse_wallet::{NearToken, Timestamp};

    const IDENTITY: SigningIdentity<'static> = &[("test_public_key", "key")];

    fn request(internal: Vec<WalletOp>, function_name: &str, args: &[u8]) -> RequestMessage {
        let mut call = FunctionCall::name(function_name).args(args);

        if FT_TRANSFER_METHODS
            .iter()
            .any(|(name, _)| *name == function_name)
        {
            call.deposit = NearToken::from_yoctonear(1);
        }

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
                        .function_call(call),
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
        let expected = r#"{
  "chain_id": "mainnet",
  "signer_id": "alice.near",
  "test_public_key": "key",
  "nonce": 1,
  "created_at": "1970-01-01T00:00:00Z",
  "timeout_secs": 60,
  "pay_for_gas": false,
  "request": {
    "internal": [
      {
        "op": "set_signature_mode",
        "payload": {
          "enable": false
        }
      }
    ],
    "external": [
      {
        "receiver_id": "token.near",
        "refund_to": null,
        "actions": [
          {
            "action": "function_call",
            "function_name": "submit",
            "deposit": "0",
            "gas": "50000000000000",
            "gas_weight": "1",
            "args": {
              "z": 1
            }
          }
        ]
      }
    ]
  }
}
"#;
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
            internal: vec![WalletOp::add_extension(
                "evil.near".parse::<AccountId>().unwrap(),
            )],
            external: vec![
                NearPromise::new("usdc.near".parse::<AccountId>().unwrap()).function_call(
                    FunctionCall::name("ft_transfer")
                        .attach_deposit(NearToken::from_yoctonear(1))
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
            shown["request"]["internal"][0]["payload"]["account_id"],
            "evil.near"
        );
        assert_eq!(
            shown["request"]["external"][0]["actions"][0]["args"],
            json!({"receiver_id": "bob.near", "amount": "1500000"})
        );
        assert!(!message.contains("eyJ"));

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
    fn nested_extensions_check_arguments_at_each_level() {
        let render_nested = |args: &[u8]| {
            let mut inner = request(Vec::new(), EXECUTE_EXTENSION, args);
            inner.request.external[0].receiver_id = "inner.near".parse().unwrap();
            request_signing_message(IDENTITY, &extension_call(&inner.request))
        };

        let leaf_args = br#"{"z":1,"a":2}"#;
        let mut leaf = call(leaf_args);
        leaf.request.internal.push(WalletOp::add_extension(
            "operator.near".parse::<AccountId>().unwrap(),
        ));
        let args = serde_json::to_vec(&json!({ "request": leaf.request })).unwrap();
        let message = render_nested(&args).unwrap();
        assert!(message.contains(
            "\nWARNING: add_extension gives operator.near full control of wallet inner.near.\n"
        ));
        let doc = document(&message);
        let outer = &doc["request"]["external"][0]["actions"][0]["args"]["request"];
        let inner = &outer["external"][0]["actions"][0]["args"]["request"];
        let shown_args = &inner["external"][0]["actions"][0]["args"];
        assert_eq!(serde_json::to_vec(shown_args).unwrap(), leaf_args);

        let leaf = call(br#"{"z":1,"0":2}"#);
        let nonportable_leaf = serde_json::to_vec(&json!({ "request": leaf.request })).unwrap();
        for malformed in [
            &br#"{"request": {}}"#[..],
            br#"{"request":{},"request":{}}"#,
            br#"{"request":{"external":[]}}"#,
            &nonportable_leaf,
        ] {
            assert!(matches!(
                render_nested(malformed),
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
            "WARNING: up to 3000000000000000000000000 yoctoNEAR sent to bob.near can be refunded to carol.near, not to this wallet.\n\n"
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
            "WARNING: up to 2 yoctoNEAR sent to bob.near can be refunded to carol.near, not to wallet token.near.\n\n"
        );
        assert_eq!(
            warnings(&extension_call(&promise(Some("token.near"), 1).into())),
            ""
        );
    }

    #[test]
    fn token_transfers_accept_only_the_method_parameters() {
        let transfer = |method: &str, args: &str| {
            request_signing_message(IDENTITY, &request(Vec::new(), method, args.as_bytes()))
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
            ("ft_transfer", "[]"),
            ("ft_transfer", "null"),
            ("ft_transfer_call", r#""bob.near""#),
        ] {
            assert!(
                matches!(transfer(method, args), Err(MessageError::TokenArgs)),
                "accepted {args}"
            );
        }
    }

    #[test]
    fn token_transfers_require_valid_fields_and_one_yocto() {
        for method in ["ft_transfer", "ft_transfer_call"] {
            let mut args = json!({"receiver_id":"bob.near","amount":"1"});
            if method == "ft_transfer_call" {
                args["msg"] = json!("");
            }
            let make =
                |args: &Value| request(Vec::new(), method, &serde_json::to_vec(args).unwrap());
            for memo in [
                None,
                Some(Value::Null),
                Some(json!("")),
                Some(json!("Привет")),
            ] {
                let mut valid = args.clone();
                if let Some(memo) = memo {
                    valid["memo"] = memo;
                }
                let msg = make(&valid);
                request_signing_message(IDENTITY, &msg).unwrap();
                request_signing_message(IDENTITY, &extension_call(&msg.request)).unwrap();
            }
            for (field, replacement) in [
                ("receiver_id", None),
                ("receiver_id", Some(Value::Null)),
                ("receiver_id", Some(json!(1))),
                ("receiver_id", Some(json!("Bad.near"))),
                ("receiver_id", Some(json!("a"))),
                ("receiver_id", Some(json!("a..near"))),
                ("memo", Some(json!(false))),
                ("memo", Some(json!([]))),
            ] {
                let mut invalid = args.clone();
                if let Some(value) = replacement {
                    invalid[field] = value;
                } else {
                    invalid.as_object_mut().unwrap().remove(field);
                }
                let msg = make(&invalid);
                for candidate in [msg.clone(), extension_call(&msg.request)] {
                    assert!(matches!(
                        request_signing_message(IDENTITY, &candidate),
                        Err(MessageError::TokenArgs)
                    ));
                }
            }
            if method == "ft_transfer_call" {
                for replacement in [None, Some(Value::Null), Some(json!(1)), Some(json!({}))] {
                    let mut invalid = args.clone();
                    if let Some(value) = replacement {
                        invalid["msg"] = value;
                    } else {
                        invalid.as_object_mut().unwrap().remove("msg");
                    }
                    let msg = make(&invalid);
                    for candidate in [msg.clone(), extension_call(&msg.request)] {
                        assert!(matches!(
                            request_signing_message(IDENTITY, &candidate),
                            Err(MessageError::TokenArgs)
                        ));
                    }
                }
            }
            for deposit in [0, 2, u128::MAX] {
                let mut msg = make(&args);
                let NearAction::FunctionCall(call) = &mut msg.request.external[0].actions[0] else {
                    unreachable!()
                };
                call.deposit = NearToken::from_yoctonear(deposit);
                // Serde omits zero deposits; decoding must still reject that default.
                let decoded: RequestMessage = serde_json::from_value(json!(msg)).unwrap();
                for candidate in [decoded, extension_call(&msg.request)] {
                    assert!(matches!(
                        request_signing_message(IDENTITY, &candidate),
                        Err(MessageError::TokenDeposit)
                    ));
                }
            }
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
    fn state_init_keys_follow_javascript_object_order() {
        // As `JSON.stringify` orders an object built from these keys: array
        // indices ascending ("9999" before "10000000"), then the other keys
        // as strings ("0100" has a leading zero, so it is not an index).
        let request: Request = serde_json::from_value(json!({
            "external": [{
                "receiver_id": "a.near",
                "actions": [{
                    "action": "deterministic_state_init",
                    "payload": {"state_init": {"V1": {
                        "code": {"account_id": "code.near"},
                        "data": {
                            "/w==": "", "0100": "", "1000": "",
                            "10000000": "", "9999": "", "AAEC": "",
                        },
                    }}},
                }],
            }],
        }))
        .unwrap();
        let msg = RequestMessage {
            request,
            ..call(b"{}")
        };
        let message = request_signing_message(IDENTITY, &msg).unwrap();
        let data =
            &document(&message)["request"]["external"][0]["actions"][0]["state_init"]["V1"]["data"];

        assert_eq!(
            data.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["1000", "9999", "10000000", "/w==", "0100", "AAEC"]
        );
    }

    #[test]
    fn warnings_and_final_lf_count_toward_the_limit() {
        let render = |padding| {
            let mut msg = call(&serde_json::to_vec(&"x".repeat(padding)).unwrap());
            msg.request.internal = vec![WalletOp::add_extension(
                "app.near".parse::<AccountId>().unwrap(),
            )];

            request_signing_message(IDENTITY, &msg)
        };
        let padding = MAX_SIGNING_MESSAGE_BYTES - render(0).unwrap().len();

        let exact = render(padding).unwrap();

        assert_eq!(exact.len(), MAX_SIGNING_MESSAGE_BYTES);
        assert!(exact.ends_with('\n'));
        assert!(matches!(render(padding + 1), Err(MessageError::TooLarge)));
    }

    #[test]
    fn total_method_name_size_is_checked_before_parsing_args() {
        let render = |name: &str| {
            let mut msg = request(Vec::new(), name, b"!");
            msg.request.external[0].actions.extend_from_within(..);

            request_signing_message(IDENTITY, &msg)
        };

        assert!(matches!(render("f"), Err(MessageError::FunctionArgs)));
        assert!(matches!(
            render(&"f".repeat(MAX_SIGNING_MESSAGE_BYTES / 2 + 1)),
            Err(MessageError::TooLarge)
        ));
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
        assert!(message.contains(
            r#""args": {
              "receiver_id": "intents.near",
              "amount": "1500000",
              "msg": "{\"memo\":\"\u041e\u043f\u043b\u0430\u0442\u0430\"}",
              "items": [
                {
                  "z": null,
                  "a": true
                }
              ]
            }
        "#
        ));
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
    fn amounts_preserve_every_base_unit_digit() {
        for units in [0, 1, 1_500_000, 9_007_199_254_740_993, u128::MAX] {
            let msg = RequestMessage {
                request: NearPromise::new("token.near".parse::<AccountId>().unwrap())
                    .transfer(NearToken::from_yoctonear(units))
                    .function_call(
                        FunctionCall::name("ft_transfer")
                            .args(format!(
                                r#"{{"receiver_id":"bob.near","amount":"{units}"}}"#
                            ))
                            .attach_deposit(NearToken::from_yoctonear(1)),
                    )
                    .function_call(
                        FunctionCall::name("submit")
                            .args(b"{}")
                            .attach_deposit(NearToken::from_yoctonear(units)),
                    )
                    .into(),
                ..call(b"{}")
            };
            let message = request_signing_message(IDENTITY, &msg).unwrap();
            let doc = document(&message);
            let actions = &doc["request"]["external"][0]["actions"];
            assert_eq!(actions[0]["amount"], units.to_string());
            assert_eq!(actions[1]["deposit"], "1");
            assert_eq!(actions[2]["deposit"], units.to_string());
            assert_eq!(actions[1]["args"]["amount"], units.to_string());
            assert_eq!(doc["request"]["external"][0]["receiver_id"], "token.near");
            assert!(message.contains("in yoctoNEAR; FT amounts are in token base units"));
            assert_eq!(near_amount(units), format!("{units} yoctoNEAR"));
        }
    }

    #[test]
    fn token_transfers_need_a_canonical_amount() {
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
                    request_signing_message(IDENTITY, &ft_transfer(Vec::new(), amount)),
                    Err(MessageError::TokenAmount)
                ),
                "accepted {amount}"
            );
        }
        assert!(matches!(
            request_signing_message(
                IDENTITY,
                &request(Vec::new(), "ft_transfer", br#"{"receiver_id":"bob.near"}"#)
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
