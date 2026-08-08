use std::{fmt, io::Read as _, num::NonZeroU32, str::FromStr, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use color_eyre::eyre::{Result, ensure};
use defuse_wallet::{
    Gas, NearPromise, NearToken, Request, RequestMessage, StateInit, Timestamp, WalletOp,
    actions::{DeterministicStateInit, FunctionCall, NearAction, Transfer},
};
use near_account_id::AccountId;

macro_rules! selectable_enum {
    ($name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum $name {
            $($variant),+
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(match self {
                    $(Self::$variant => $label),+
                })
            }
        }
    };
}

selectable_enum! {
    RequestItemChoice {
        AddInternal => "Add an internal wallet operation",
        AddExternal => "Add an external NEAR promise",
        Finish => "Finish building request contents",
    }
}

selectable_enum! {
    WalletOperationChoice {
        EnableSignature => "Enable signature authentication",
        DisableSignature => "Disable signature authentication",
        AddExtension => "Add an extension",
        RemoveExtension => "Remove an extension",
        Back => "Back without adding an operation",
    }
}

selectable_enum! {
    PromiseActionChoice {
        FunctionCall => "Add a function-call action",
        Transfer => "Add a NEAR transfer action",
        DeterministicStateInit => "Add a deterministic-state-init action",
        Finish => "Finish this external promise",
        Discard => "Discard this external promise",
    }
}

selectable_enum! {
    FunctionArgumentsChoice {
        None => "No function arguments",
        Json => "JSON arguments (serialized as compact UTF-8 JSON)",
        Utf8 => "Raw UTF-8 text",
        Base64 => "Base64-encoded bytes",
        Back => "Back without adding the function call",
    }
}

/// Build a complete request message through terminal prompts.
pub fn prompt_request_message() -> Result<Option<RequestMessage>> {
    loop {
        let Some(chain_id) = prompt_value_default(
            "NEAR chain ID",
            NonEmptyString::new("mainnet").expect("hardcoded value is non-empty"),
        )?
        else {
            return Ok(None);
        };
        let Some(signer_id) = prompt_value::<AccountIdInput>(
            "Wallet-contract account ID (RequestMessage.signer_id)",
        )?
        else {
            return Ok(None);
        };
        let Some(nonce) = prompt_value::<u32>("Request nonce (u32)")? else {
            return Ok(None);
        };
        let Some(timeout_secs) = prompt_value_default(
            "Request timeout in seconds",
            NonZeroU32::new(3_600).expect("hardcoded timeout is nonzero"),
        )?
        else {
            return Ok(None);
        };

        let default_created_at = TimestampInput(default_created_at(Timestamp::now(), timeout_secs));
        let Some(created_at) =
            prompt_value_default("Creation time (RFC-3339)", default_created_at)?
        else {
            return Ok(None);
        };
        let Some(request) = prompt_request_contents(&signer_id.0)? else {
            return Ok(None);
        };

        let message = assemble_request_message(
            chain_id.value,
            signer_id.0,
            nonce,
            created_at.0,
            timeout_secs.get(),
            request,
        );
        eprintln!(
            "\nRequestMessage preview:\n{}\n",
            serde_json::to_string_pretty(&message)?
        );

        let Some(confirmed) = prompt_confirm("Use this RequestMessage?", true)? else {
            return Ok(None);
        };
        if confirmed {
            return Ok(Some(message));
        }
        eprintln!("Restarting the RequestMessage builder.\n");
    }
}

fn default_created_at(now: Timestamp, timeout_secs: NonZeroU32) -> Timestamp {
    let timeout = Duration::from_secs(u64::from(timeout_secs.get()));
    let blockchain_lag = Duration::from_mins(1).min(timeout / 5);
    now.saturating_sub_unsigned(blockchain_lag)
}

fn assemble_request_message(
    chain_id: String,
    signer_id: AccountId,
    nonce: u32,
    created_at: Timestamp,
    timeout_secs: u32,
    request: Request,
) -> RequestMessage {
    RequestMessage {
        // Paying gas from wallet state is currently unsupported by the contract.
        pay_for_gas: false,
        chain_id,
        signer_id,
        nonce,
        created_at,
        timeout: Duration::from_secs(u64::from(timeout_secs)),
        request,
    }
}

fn prompt_request_contents(wallet_signer_id: &AccountId) -> Result<Option<Request>> {
    let mut request = Request::new();

    eprintln!(
        "Internal operations execute before all external promises; ordering is preserved within each group. Internal-operation validity depends on current wallet state."
    );

    loop {
        let message = format!(
            "Request contents ({} internal operation(s), {} external promise(s))",
            request.internal.len(),
            request.external.len(),
        );
        let Some(choice) = prompt_select(
            &message,
            &[
                RequestItemChoice::AddInternal,
                RequestItemChoice::AddExternal,
                RequestItemChoice::Finish,
            ],
        )?
        else {
            return Ok(None);
        };

        match choice {
            RequestItemChoice::AddInternal => {
                if let Some(operation) = prompt_wallet_operation()? {
                    request.internal.push(operation);
                }
            }
            RequestItemChoice::AddExternal => {
                if let Some(promise) = prompt_external_promise(wallet_signer_id)? {
                    request.external.push(promise);
                }
            }
            RequestItemChoice::Finish => return Ok(Some(request)),
        }
    }
}

fn prompt_wallet_operation() -> Result<Option<WalletOp>> {
    let Some(choice) = prompt_select(
        "Internal wallet operation",
        &[
            WalletOperationChoice::EnableSignature,
            WalletOperationChoice::DisableSignature,
            WalletOperationChoice::AddExtension,
            WalletOperationChoice::RemoveExtension,
            WalletOperationChoice::Back,
        ],
    )?
    else {
        return Ok(None);
    };

    match choice {
        WalletOperationChoice::EnableSignature => Ok(Some(WalletOp::enable_signature())),
        WalletOperationChoice::DisableSignature => Ok(Some(WalletOp::disable_signature())),
        WalletOperationChoice::AddExtension => {
            let Some(account_id) = prompt_value::<AccountIdInput>("Extension account ID to add")?
            else {
                return Ok(None);
            };
            Ok(Some(WalletOp::add_extension(account_id.0)))
        }
        WalletOperationChoice::RemoveExtension => {
            let Some(account_id) =
                prompt_value::<AccountIdInput>("Extension account ID to remove")?
            else {
                return Ok(None);
            };
            Ok(Some(WalletOp::remove_extension(account_id.0)))
        }
        WalletOperationChoice::Back => Ok(None),
    }
}

fn prompt_external_promise(wallet_signer_id: &AccountId) -> Result<Option<NearPromise>> {
    let receiver_id = loop {
        let Some(receiver_id) =
            prompt_value::<AccountIdInput>("External promise receiver account ID")?
        else {
            return Ok(None);
        };
        if receiver_id.0 == *wallet_signer_id {
            eprintln!("Wallet self-calls are not allowed; choose another receiver.");
        } else {
            break receiver_id;
        }
    };
    let Some(set_refund_to) = prompt_confirm("Set a refund account for failed deposits?", false)?
    else {
        return Ok(None);
    };
    let refund_to = if set_refund_to {
        let Some(account_id) = prompt_value::<AccountIdInput>("Refund account ID")? else {
            return Ok(None);
        };
        Some(account_id.0)
    } else {
        None
    };

    let mut actions = Vec::new();
    loop {
        let message = format!("External promise actions ({} added)", actions.len());
        let choices = [
            PromiseActionChoice::FunctionCall,
            PromiseActionChoice::Transfer,
            PromiseActionChoice::DeterministicStateInit,
            PromiseActionChoice::Finish,
            PromiseActionChoice::Discard,
        ];
        let Some(choice) = prompt_select(&message, &choices)? else {
            return Ok(None);
        };

        match choice {
            PromiseActionChoice::FunctionCall => {
                if let Some(action) = prompt_function_call()? {
                    actions.push(action);
                }
            }
            PromiseActionChoice::Transfer => {
                if let Some(action) = prompt_transfer()? {
                    actions.push(action);
                }
            }
            PromiseActionChoice::DeterministicStateInit => {
                if let Some(action) = prompt_deterministic_state_init(&receiver_id.0)? {
                    actions.push(action);
                }
            }
            PromiseActionChoice::Finish if actions.is_empty() => {
                eprintln!("An external promise must contain at least one action.");
            }
            PromiseActionChoice::Finish => {
                return Ok(Some(build_external_promise(
                    wallet_signer_id,
                    receiver_id.0,
                    refund_to,
                    actions,
                )?));
            }
            PromiseActionChoice::Discard => return Ok(None),
        }
    }
}

fn build_external_promise(
    wallet_signer_id: &AccountId,
    receiver_id: AccountId,
    refund_to: Option<AccountId>,
    actions: Vec<NearAction>,
) -> Result<NearPromise> {
    ensure!(
        receiver_id != *wallet_signer_id,
        "wallet self-calls are not allowed"
    );
    ensure!(
        !actions.is_empty(),
        "an external promise must contain at least one action"
    );
    Ok(NearPromise {
        receiver_id,
        refund_to,
        actions,
    })
}

fn prompt_function_call() -> Result<Option<NearAction>> {
    let Some(function_name) = prompt_value::<NonEmptyString>("Function name")? else {
        return Ok(None);
    };
    let Some(args) = prompt_function_arguments()? else {
        return Ok(None);
    };
    let Some(deposit) = prompt_value_default("Attached deposit", TokenInput(NearToken::ZERO))?
    else {
        return Ok(None);
    };
    let Some(gas) = prompt_value_default("Minimum gas", GasInput(Gas::from_tgas(50)))? else {
        return Ok(None);
    };
    let Some(gas_weight) = prompt_value_default("Unused-gas weight", 1_u64)? else {
        return Ok(None);
    };

    Ok(Some(build_function_call(
        function_name.value,
        args,
        deposit.0,
        gas.0,
        gas_weight,
    )))
}

fn build_function_call(
    function_name: String,
    args: Vec<u8>,
    deposit: NearToken,
    gas: Gas,
    gas_weight: u64,
) -> NearAction {
    FunctionCall::name(function_name)
        .args(args)
        .attach_deposit(deposit)
        .gas(gas)
        .unused_gas_weight(gas_weight)
        .into()
}

fn prompt_function_arguments() -> Result<Option<Vec<u8>>> {
    let Some(choice) = prompt_select(
        "Function-call argument encoding",
        &[
            FunctionArgumentsChoice::None,
            FunctionArgumentsChoice::Json,
            FunctionArgumentsChoice::Utf8,
            FunctionArgumentsChoice::Base64,
            FunctionArgumentsChoice::Back,
        ],
    )?
    else {
        return Ok(None);
    };

    match choice {
        FunctionArgumentsChoice::None => Ok(Some(Vec::new())),
        FunctionArgumentsChoice::Json => {
            let Some(value) = prompt_value_default(
                "JSON arguments (inline JSON or @FILE)",
                JsonBytesInput::from_str("{}").expect("hardcoded JSON is valid"),
            )?
            else {
                return Ok(None);
            };
            Ok(Some(value.bytes))
        }
        FunctionArgumentsChoice::Utf8 => {
            prompt_text("Raw UTF-8 function arguments").map(|value| value.map(String::into_bytes))
        }
        FunctionArgumentsChoice::Base64 => {
            let Some(value) = prompt_value::<Base64BytesInput>("Base64-encoded arguments")? else {
                return Ok(None);
            };
            Ok(Some(value.bytes))
        }
        FunctionArgumentsChoice::Back => Ok(None),
    }
}

fn prompt_transfer() -> Result<Option<NearAction>> {
    let Some(amount) = prompt_value::<TokenInput>("Amount to transfer (for example, 1 NEAR)")?
    else {
        return Ok(None);
    };
    Ok(Some(Transfer { amount: amount.0 }.into()))
}

fn prompt_deterministic_state_init(receiver_id: &AccountId) -> Result<Option<NearAction>> {
    eprintln!(
        "Enter typed StateInit JSON. Example: {{\"V1\":{{\"code\":{{\"account_id\":\"global.near\"}},\"data\":{{}}}}}}"
    );
    let state_init = loop {
        let Some(state_init) = prompt_value::<StateInitInput>("StateInit JSON (inline or @FILE)")?
        else {
            return Ok(None);
        };
        match validate_state_init_receiver(&state_init.state_init, receiver_id) {
            Ok(()) => break state_init,
            Err(error) => eprintln!(
                "{error}. Enter a StateInit matching this receiver, or press Escape to go back."
            ),
        }
    };
    let Some(deposit) = prompt_value_default("Attached deposit", TokenInput(NearToken::ZERO))?
    else {
        return Ok(None);
    };

    Ok(Some(
        DeterministicStateInit::new(state_init.state_init)
            .deposit(deposit.0)
            .into(),
    ))
}

fn validate_state_init_receiver(state_init: &StateInit, receiver_id: &AccountId) -> Result<()> {
    let derived_account_id = state_init.derive_account_id();
    ensure!(
        derived_account_id == *receiver_id,
        "StateInit derives receiver '{derived_account_id}', but this promise targets '{receiver_id}'"
    );
    Ok(())
}

fn prompt_value<T>(message: &str) -> Result<Option<T>>
where
    T: Clone + fmt::Display + FromStr,
    T::Err: fmt::Display,
{
    map_prompt(inquire::CustomType::<T>::new(message).prompt())
}

fn prompt_value_default<T>(message: &str, default: T) -> Result<Option<T>>
where
    T: Clone + fmt::Display + FromStr,
    T::Err: fmt::Display,
{
    map_prompt(
        inquire::CustomType::<T>::new(message)
            .with_default(default)
            .prompt(),
    )
}

fn prompt_text(message: &str) -> Result<Option<String>> {
    map_prompt(inquire::Text::new(message).prompt())
}

fn prompt_confirm(message: &str, default: bool) -> Result<Option<bool>> {
    map_prompt(
        inquire::Confirm::new(message)
            .with_default(default)
            .prompt(),
    )
}

fn prompt_select<T>(message: &str, choices: &[T]) -> Result<Option<T>>
where
    T: Clone + fmt::Display,
{
    map_prompt(inquire::Select::new(message, choices.to_vec()).prompt())
}

fn map_prompt<T>(result: inquire::error::InquireResult<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(inquire::error::InquireError::OperationCanceled) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn read_string_source(source: &str) -> Result<String, String> {
    match source {
        "@-" => {
            let mut value = String::new();
            std::io::stdin()
                .lock()
                .read_to_string(&mut value)
                .map_err(|error| format!("failed to read stdin: {error}"))?;
            Ok(value)
        }
        _ => source.strip_prefix('@').map_or_else(
            || Ok(source.to_owned()),
            |path| {
                std::fs::read_to_string(path)
                    .map_err(|error| format!("failed to read {path}: {error}"))
            },
        ),
    }
}

#[derive(Debug, Clone)]
struct NonEmptyString {
    value: String,
}

impl NonEmptyString {
    fn new(value: &str) -> Result<Self, String> {
        value.parse()
    }
}

impl fmt::Display for NonEmptyString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value)
    }
}

impl FromStr for NonEmptyString {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.trim().is_empty() {
            return Err("value cannot be empty".to_owned());
        }
        Ok(Self {
            value: value.to_owned(),
        })
    }
}

#[derive(Debug, Clone)]
struct AccountIdInput(AccountId);

impl fmt::Display for AccountIdInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for AccountIdInput {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse()
            .map(Self)
            .map_err(|error| format!("invalid NEAR account ID: {error}"))
    }
}

#[derive(Debug, Clone, Copy)]
struct TimestampInput(Timestamp);

impl fmt::Display for TimestampInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for TimestampInput {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let timestamp: Timestamp = value
            .parse()
            .map_err(|error| format!("invalid RFC-3339 timestamp: {error}"))?;
        u64::try_from(timestamp.as_nanos()).map_err(|_| {
            "timestamp must fit the wallet's unsigned 64-bit nanosecond encoding".to_owned()
        })?;
        Ok(Self(timestamp))
    }
}

#[derive(Debug, Clone, Copy)]
struct TokenInput(NearToken);

impl fmt::Display for TokenInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for TokenInput {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse()
            .map(Self)
            .map_err(|error| format!("invalid NEAR token amount: {error}"))
    }
}

#[derive(Debug, Clone, Copy)]
struct GasInput(Gas);

impl fmt::Display for GasInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for GasInput {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse()
            .map(Self)
            .map_err(|error| format!("invalid gas amount: {error}"))
    }
}

#[derive(Debug, Clone)]
struct JsonBytesInput {
    source: String,
    bytes: Vec<u8>,
}

impl fmt::Display for JsonBytesInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.source)
    }
}

impl FromStr for JsonBytesInput {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let json = read_string_source(source)?;
        let value: serde_json::Value =
            serde_json::from_str(&json).map_err(|error| format!("invalid JSON: {error}"))?;
        let bytes = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
        Ok(Self {
            source: source.to_owned(),
            bytes,
        })
    }
}

#[derive(Debug, Clone)]
struct Base64BytesInput {
    source: String,
    bytes: Vec<u8>,
}

impl fmt::Display for Base64BytesInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.source)
    }
}

impl FromStr for Base64BytesInput {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let bytes = BASE64
            .decode(source)
            .map_err(|error| format!("invalid base64: {error}"))?;
        Ok(Self {
            source: source.to_owned(),
            bytes,
        })
    }
}

#[derive(Debug, Clone)]
struct StateInitInput {
    source: String,
    state_init: StateInit,
}

impl fmt::Display for StateInitInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.source)
    }
}

impl FromStr for StateInitInput {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let json = read_string_source(source)?;
        let state_init = serde_json::from_str(&json)
            .map_err(|error| format!("invalid typed StateInit JSON: {error}"))?;
        Ok(Self {
            source: source.to_owned(),
            state_init,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_request_with_internal_and_external_operations() {
        let wallet_signer_id: AccountId = "wallet.near".parse().unwrap();
        let state_init: StateInit =
            serde_json::from_str(r#"{"V1":{"code":{"account_id":"global.near"},"data":{}}}"#)
                .unwrap();
        let receiver_id = state_init.derive_account_id();
        let external = build_external_promise(
            &wallet_signer_id,
            receiver_id,
            Some("refund.near".parse().unwrap()),
            vec![
                DeterministicStateInit::new(state_init).into(),
                build_function_call(
                    "store".to_owned(),
                    br#"{"value":1}"#.to_vec(),
                    NearToken::from_yoctonear(1),
                    Gas::from_tgas(30),
                    2,
                ),
                Transfer {
                    amount: NearToken::from_near(1),
                }
                .into(),
            ],
        )
        .unwrap();
        let request = Request::new()
            .internal([
                WalletOp::add_extension("extension.near".parse::<AccountId>().unwrap()),
                WalletOp::disable_signature(),
                WalletOp::enable_signature(),
                WalletOp::remove_extension("extension.near".parse::<AccountId>().unwrap()),
            ])
            .external([external]);
        let message = assemble_request_message(
            "mainnet".to_owned(),
            wallet_signer_id,
            7,
            "2026-08-08T10:00:00Z".parse().unwrap(),
            3_600,
            request,
        );

        let json = serde_json::to_value(message).unwrap();
        assert_eq!(json["request"]["internal"].as_array().unwrap().len(), 4);
        assert_eq!(json["request"]["internal"][0]["op"], "add_extension");
        assert_eq!(json["request"]["internal"][1]["op"], "set_signature_mode");
        assert_eq!(json["request"]["internal"][2]["op"], "set_signature_mode");
        assert_eq!(json["request"]["internal"][3]["op"], "remove_extension");
        assert_eq!(json["request"]["external"].as_array().unwrap().len(), 1);
        assert_eq!(
            json["request"]["external"][0]["actions"][0]["action"],
            "deterministic_state_init"
        );
        assert_eq!(
            json["request"]["external"][0]["actions"][1]["action"],
            "function_call"
        );
        assert_eq!(
            json["request"]["external"][0]["actions"][2]["action"],
            "transfer"
        );
    }

    #[test]
    fn parses_every_function_argument_encoding() {
        let json: JsonBytesInput = "{ \"answer\": 42 }".parse().unwrap();
        assert_eq!(json.bytes, br#"{"answer":42}"#);

        let base64: Base64BytesInput = "aGVsbG8=".parse().unwrap();
        assert_eq!(base64.bytes, b"hello");
    }

    #[test]
    fn parses_typed_state_init_json() {
        let state: StateInitInput = r#"{"V1":{"code":{"account_id":"global.near"},"data":{}}}"#
            .parse()
            .unwrap();
        assert!(matches!(state.state_init, StateInit::V1(_)));
    }

    #[test]
    fn validates_external_promise_invariants() {
        let wallet: AccountId = "wallet.near".parse().unwrap();
        let receiver: AccountId = "receiver.near".parse().unwrap();

        assert!(build_external_promise(&wallet, wallet.clone(), None, vec![]).is_err());
        assert!(build_external_promise(&wallet, receiver.clone(), None, vec![]).is_err());

        let state: StateInit =
            serde_json::from_str(r#"{"V1":{"code":{"account_id":"global.near"},"data":{}}}"#)
                .unwrap();
        assert!(validate_state_init_receiver(&state, &receiver).is_err());
        let derived = state.derive_account_id();
        assert!(validate_state_init_receiver(&state, &derived).is_ok());

        let transfer_before_state_init = vec![
            Transfer {
                amount: NearToken::from_near(1),
            }
            .into(),
            DeterministicStateInit::new(state).into(),
        ];
        assert!(build_external_promise(&wallet, derived, None, transfer_before_state_init).is_ok());
    }

    #[test]
    fn short_timeout_default_remains_inside_validity_window() {
        let now: Timestamp = "2026-08-08T10:00:00Z".parse().unwrap();
        let timeout = NonZeroU32::new(10).unwrap();
        let created_at = default_created_at(now, timeout);

        assert_eq!(
            now.duration_since(created_at).unwrap(),
            Duration::from_secs(2)
        );
        assert!(now.duration_since(created_at).unwrap() <= Duration::from_secs(10));
    }

    #[test]
    fn timestamp_input_must_fit_canonical_borsh_encoding() {
        assert!("1969-12-31T23:59:59Z".parse::<TimestampInput>().is_err());

        let max = Timestamp::from_nanos(i128::from(u64::MAX)).unwrap();
        assert!(max.to_string().parse::<TimestampInput>().is_ok());

        let too_large = Timestamp::from_nanos(i128::from(u64::MAX) + 1).unwrap();
        assert!(too_large.to_string().parse::<TimestampInput>().is_err());
    }

    #[test]
    fn escape_goes_back_but_interrupt_aborts() {
        assert!(
            map_prompt::<()>(Err(inquire::error::InquireError::OperationCanceled))
                .unwrap()
                .is_none()
        );
        assert!(map_prompt::<()>(Err(inquire::error::InquireError::OperationInterrupted)).is_err());
    }

    #[test]
    fn token_and_gas_inputs_require_units() {
        assert_eq!(
            "1 NEAR".parse::<TokenInput>().unwrap().0,
            NearToken::from_near(1)
        );
        assert_eq!("50 Tgas".parse::<GasInput>().unwrap().0, Gas::from_tgas(50));
        assert!("1".parse::<TokenInput>().is_err());
        assert!("50".parse::<GasInput>().is_err());
    }
}
