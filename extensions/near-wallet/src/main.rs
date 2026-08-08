use std::{
    fmt,
    io::{self, Write as _},
    path::PathBuf,
    str::FromStr,
};

use color_eyre::eyre::{Result, eyre};
use defuse_crypto::ed25519::Ed25519PublicKey;
use defuse_wallet::RequestMessage;
use interactive_clap::ResultFromCli;
use near_account_id::AccountId;
use near_wallet::{read_request_message, sign_request_with_near, wallet_nep413_payload};
use serde::Serialize;
use strum::{EnumDiscriminants, EnumIter, EnumMessage};

mod request_builder;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = ())]
struct WalletCli {
    #[interactive_clap(subcommand)]
    action: WalletAction,
}

#[derive(Debug, EnumDiscriminants, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = ())]
#[interactive_clap(disable_back)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
/// What do you want to do with a wallet-contract request?
#[allow(dead_code)] // interactive-clap constructs the generated CLI variants instead.
enum WalletAction {
    #[strum_discriminants(strum(
        message = "payload  - Convert a RequestMessage into its deterministic NEP-413 payload"
    ))]
    /// Convert a wallet `RequestMessage` to its deterministic NEP-413 payload
    Payload(PayloadCommand),
    #[strum_discriminants(strum(
        message = "sign     - Sign a RequestMessage and print w_execute_signed JSON arguments"
    ))]
    /// Sign through near-cli and print `w_execute_signed` JSON arguments
    Sign(SignCommand),
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = ())]
#[interactive_clap(output_context = PayloadOutputContext)]
struct PayloadCommand {
    #[interactive_clap(skip_default_input_arg)]
    /// `RequestMessage` as inline JSON, @FILE, or @-; omit to load/build interactively
    request_message: RequestMessageInput,

    #[interactive_clap(long)]
    #[interactive_clap(skip_interactive_input)]
    /// Optional callback URL for external/manual signing; unusable with `w_execute_signed`
    callback_url: Option<String>,

    #[interactive_clap(long)]
    /// Pretty-print the JSON output
    pretty: bool,
}

impl PayloadCommand {
    #[allow(clippy::trivially_copy_pass_by_ref)] // Signature required by interactive-clap.
    fn input_request_message(_context: &()) -> Result<Option<RequestMessageInput>> {
        prompt_request_message_input()
    }
}

#[derive(Debug, Clone)]
struct PayloadOutputContext;

impl PayloadOutputContext {
    fn from_previous_context(
        _previous_context: (),
        scope: &<PayloadCommand as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        let mut payload = wallet_nep413_payload(scope.request_message.message.clone());
        payload.callback_url.clone_from(&scope.callback_url);
        write_json(&payload, scope.pretty)?;
        Ok(Self)
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = ())]
#[interactive_clap(output_context = SignContext)]
struct SignCommand {
    #[interactive_clap(skip_default_input_arg)]
    /// `RequestMessage` as inline JSON, @FILE, or @-; omit to load/build interactively
    request_message: RequestMessageInput,

    #[interactive_clap(
        long = "sign-as",
        visible_alias = "signer-account-id",
        value_name = "ACCOUNT_ID"
    )]
    #[interactive_clap(skip_default_input_arg)]
    /// NEAR access-key account near-cli should sign as (not `RequestMessage.signer_id`)
    sign_as: SignerAccountInput,

    #[interactive_clap(long, value_name = "ed25519:PUBLIC_KEY")]
    #[interactive_clap(skip_interactive_input)]
    /// Require near-cli to use this wallet's fixed Ed25519 public key
    expected_public_key: Option<ExpectedPublicKeyInput>,

    #[interactive_clap(long, default_value = "near", value_name = "PATH")]
    /// near-cli-rs executable to invoke
    near_command: PathInput,

    #[interactive_clap(long)]
    /// Pretty-print the `w_execute_signed` JSON arguments
    pretty: bool,

    #[interactive_clap(subcommand)]
    sign_with: SignWith,
}

impl SignCommand {
    #[allow(clippy::trivially_copy_pass_by_ref)] // Signature required by interactive-clap.
    fn input_request_message(_context: &()) -> Result<Option<RequestMessageInput>> {
        prompt_request_message_input()
    }

    #[allow(clippy::trivially_copy_pass_by_ref)] // Signature required by interactive-clap.
    fn input_sign_as(_context: &()) -> Result<Option<SignerAccountInput>> {
        prompt_custom_type(
            "NEAR access-key account to sign as (distinct from the wallet contract signer_id)",
        )
    }
}

#[derive(Debug, Clone)]
struct SignContext {
    request_message: RequestMessage,
    sign_as: AccountId,
    expected_public_key: Option<Ed25519PublicKey>,
    near_command: PathBuf,
    pretty: bool,
}

impl SignContext {
    #[allow(clippy::unnecessary_wraps)] // Signature required by interactive-clap.
    fn from_previous_context(
        _previous_context: (),
        scope: &<SignCommand as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        if scope.expected_public_key.is_none() {
            eprintln!(
                "Warning: no --expected-public-key guard was supplied; the returned key will still be checked as Ed25519 and the signature will be verified locally."
            );
        }

        Ok(Self {
            request_message: scope.request_message.message.clone(),
            sign_as: scope.sign_as.account_id.clone(),
            expected_public_key: scope.expected_public_key.as_ref().map(|key| key.public_key),
            near_command: scope.near_command.path.clone(),
            pretty: scope.pretty,
        })
    }
}

#[derive(Debug, EnumDiscriminants, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = SignContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
/// How do you want near-cli to sign the wallet request?
#[allow(dead_code)] // interactive-clap constructs the generated CLI variants instead.
enum SignWith {
    #[strum_discriminants(strum(
        message = "sign-with-keychain               - Use a key in the secure keychain"
    ))]
    /// Sign with a key saved in the secure keychain
    SignWithKeychain(SignWithKeychain),
    #[strum_discriminants(strum(
        message = "sign-with-legacy-keychain        - Use a key in the legacy credentials directory"
    ))]
    /// Sign with a key saved in the legacy credentials directory
    SignWithLegacyKeychain(SignWithLegacyKeychain),
    #[strum_discriminants(strum(
        message = "sign-with-ledger                 - Use a Ledger connected over USB"
    ))]
    /// Sign with a Ledger hardware wallet
    SignWithLedger(SignWithLedger),
    #[strum_discriminants(strum(
        message = "sign-with-plaintext-private-key  - Use a plaintext Ed25519 private key"
    ))]
    /// Sign with a plaintext private key
    SignWithPlaintextPrivateKey(SignWithPlaintextPrivateKey),
    #[strum_discriminants(strum(
        message = "sign-with-access-key-file        - Use a NEAR access-key JSON file"
    ))]
    /// Sign with a NEAR account access-key JSON file
    SignWithAccessKeyFile(SignWithAccessKeyFile),
    #[strum_discriminants(strum(
        message = "sign-with-seed-phrase            - Derive a key from a seed phrase"
    ))]
    /// Sign with a key derived from a seed phrase
    SignWithSeedPhrase(SignWithSeedPhrase),
    #[strum_discriminants(strum(
        message = "custom                           - Pass another signing tail to near-cli"
    ))]
    /// Pass an arbitrary signing-command tail to near-cli
    Custom(CustomSigningArguments),
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = SignContext)]
#[interactive_clap(output_context = KeychainOutputContext)]
/// Sign with a key saved in the secure keychain.
struct SignWithKeychain {
    /// Name of the configured NEAR network (for example, mainnet or testnet)
    network_name: String,
}

#[derive(Debug, Clone)]
struct KeychainOutputContext;

impl KeychainOutputContext {
    fn from_previous_context(
        context: SignContext,
        scope: &<SignWithKeychain as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        execute_sign(
            context,
            &[
                "sign-with-keychain".to_owned(),
                "network-config".to_owned(),
                scope.network_name.clone(),
            ],
        )?;
        Ok(Self)
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = SignContext)]
#[interactive_clap(output_context = LegacyKeychainOutputContext)]
/// Sign with a key saved in the legacy credentials directory.
struct SignWithLegacyKeychain {
    /// Name of the configured NEAR network (for example, mainnet or testnet)
    network_name: String,
}

#[derive(Debug, Clone)]
struct LegacyKeychainOutputContext;

impl LegacyKeychainOutputContext {
    fn from_previous_context(
        context: SignContext,
        scope: &<SignWithLegacyKeychain as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        execute_sign(
            context,
            &[
                "sign-with-legacy-keychain".to_owned(),
                "network-config".to_owned(),
                scope.network_name.clone(),
            ],
        )?;
        Ok(Self)
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = SignContext)]
#[interactive_clap(output_context = LedgerContext)]
/// Sign with a Ledger hardware wallet.
struct SignWithLedger {
    #[interactive_clap(long, default_value = "m/44'/397'/0'", value_name = "HD_PATH")]
    /// SLIP-0010 derivation path used by the Ledger device
    seed_phrase_hd_path: String,

    #[interactive_clap(subcommand)]
    connection: LedgerConnection,
}

#[derive(Debug, Clone)]
struct LedgerContext {
    sign_context: SignContext,
    seed_phrase_hd_path: String,
}

impl LedgerContext {
    #[allow(clippy::unnecessary_wraps)] // Signature required by interactive-clap.
    fn from_previous_context(
        context: SignContext,
        scope: &<SignWithLedger as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        Ok(Self {
            sign_context: context,
            seed_phrase_hd_path: scope.seed_phrase_hd_path.clone(),
        })
    }
}

#[derive(Debug, EnumDiscriminants, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = LedgerContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
/// Select the Ledger connection type
#[allow(dead_code)] // interactive-clap constructs the generated CLI variants instead.
enum LedgerConnection {
    #[strum_discriminants(strum(message = "usb  - Connect to Ledger over USB"))]
    /// Connect to Ledger over USB
    Usb(LedgerUsb),
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = LedgerContext)]
#[interactive_clap(output_context = LedgerUsbOutputContext)]
/// Connect to a Ledger hardware wallet over USB.
struct LedgerUsb {}

#[derive(Debug, Clone)]
struct LedgerUsbOutputContext;

impl LedgerUsbOutputContext {
    fn from_previous_context(
        context: LedgerContext,
        _scope: &<LedgerUsb as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        let hd_path = context.seed_phrase_hd_path;
        execute_sign(
            context.sign_context,
            &[
                "sign-with-ledger".to_owned(),
                "--seed-phrase-hd-path".to_owned(),
                hd_path,
                "usb".to_owned(),
            ],
        )?;
        Ok(Self)
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = SignContext)]
#[interactive_clap(output_context = PrivateKeyOutputContext)]
/// Sign with a plaintext private key.
struct SignWithPlaintextPrivateKey {
    #[interactive_clap(skip_default_input_arg)]
    /// Ed25519 private key (entered without terminal echo in interactive mode)
    private_key: SecretInput,
}

impl SignWithPlaintextPrivateKey {
    fn input_private_key(_context: &SignContext) -> Result<Option<SecretInput>> {
        prompt_secret("Ed25519 private key")
    }
}

#[derive(Debug, Clone)]
struct PrivateKeyOutputContext;

impl PrivateKeyOutputContext {
    fn from_previous_context(
        context: SignContext,
        scope: &<SignWithPlaintextPrivateKey as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        execute_sign(
            context,
            &[
                "sign-with-plaintext-private-key".to_owned(),
                scope.private_key.secret.clone(),
            ],
        )?;
        Ok(Self)
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = SignContext)]
#[interactive_clap(output_context = AccessKeyFileOutputContext)]
/// Sign with a NEAR account access-key JSON file.
struct SignWithAccessKeyFile {
    /// Location of the NEAR account access-key JSON file
    file_path: PathInput,
}

#[derive(Debug, Clone)]
struct AccessKeyFileOutputContext;

impl AccessKeyFileOutputContext {
    fn from_previous_context(
        context: SignContext,
        scope: &<SignWithAccessKeyFile as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        execute_sign(
            context,
            &[
                "sign-with-access-key-file".to_owned(),
                scope.file_path.to_string(),
            ],
        )?;
        Ok(Self)
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = SignContext)]
#[interactive_clap(output_context = SeedPhraseOutputContext)]
/// Sign with a key derived from a seed phrase.
struct SignWithSeedPhrase {
    #[interactive_clap(skip_default_input_arg)]
    /// Seed phrase (entered without terminal echo in interactive mode)
    master_seed_phrase: SecretInput,

    #[interactive_clap(long, default_value = "m/44'/397'/0'", value_name = "HD_PATH")]
    /// SLIP-0010 derivation path
    seed_phrase_hd_path: String,
}

impl SignWithSeedPhrase {
    fn input_master_seed_phrase(_context: &SignContext) -> Result<Option<SecretInput>> {
        prompt_secret("Seed phrase")
    }
}

#[derive(Debug, Clone)]
struct SeedPhraseOutputContext;

impl SeedPhraseOutputContext {
    fn from_previous_context(
        context: SignContext,
        scope: &<SignWithSeedPhrase as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        execute_sign(
            context,
            &[
                "sign-with-seed-phrase".to_owned(),
                scope.master_seed_phrase.secret.clone(),
                "--seed-phrase-hd-path".to_owned(),
                scope.seed_phrase_hd_path.clone(),
            ],
        )?;
        Ok(Self)
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = SignContext)]
#[interactive_clap(output_context = CustomSigningOutputContext)]
/// Pass an arbitrary signing-command tail to near-cli.
struct CustomSigningArguments {
    #[interactive_clap(skip_default_input_arg)]
    /// Shell-style near-cli signing tail, for example: sign-with-keychain network-config mainnet
    signing_arguments: ShellArgumentsInput,
}

impl CustomSigningArguments {
    fn input_signing_arguments(_context: &SignContext) -> Result<Option<ShellArgumentsInput>> {
        prompt_custom_type("near-cli signing tail")
    }
}

#[derive(Debug, Clone)]
struct CustomSigningOutputContext;

impl CustomSigningOutputContext {
    fn from_previous_context(
        context: SignContext,
        scope: &<CustomSigningArguments as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> Result<Self> {
        execute_sign(context, &scope.signing_arguments.arguments)?;
        Ok(Self)
    }
}

fn execute_sign(context: SignContext, signing_arguments: &[String]) -> Result<()> {
    let call_args = sign_request_with_near(
        &context.near_command,
        context.request_message,
        &context.sign_as,
        context.expected_public_key.as_ref(),
        signing_arguments,
    )
    .map_err(|error| eyre!("{error:#}"))?;
    write_json(&call_args, context.pretty)
}

fn write_json(value: &impl Serialize, pretty: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    if pretty {
        serde_json::to_writer_pretty(&mut stdout, value)?;
    } else {
        serde_json::to_writer(&mut stdout, value)?;
    }
    writeln!(stdout)?;
    Ok(())
}

fn prompt_custom_type<T>(message: &str) -> Result<Option<T>>
where
    T: Clone + fmt::Display + FromStr,
    T::Err: fmt::Display,
{
    match inquire::CustomType::<T>::new(message).prompt() {
        Ok(value) => Ok(Some(value)),
        Err(
            inquire::error::InquireError::OperationCanceled
            | inquire::error::InquireError::OperationInterrupted,
        ) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

#[derive(Debug, Clone, Copy)]
enum RequestSource {
    Existing,
    Build,
}

impl fmt::Display for RequestSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Existing => "Use existing RequestMessage JSON (inline, @FILE, or @-)",
            Self::Build => "Build a new RequestMessage interactively",
        })
    }
}

fn prompt_request_message_input() -> Result<Option<RequestMessageInput>> {
    let source = match inquire::Select::new(
        "RequestMessage source",
        vec![RequestSource::Existing, RequestSource::Build],
    )
    .prompt()
    {
        Ok(source) => source,
        Err(
            inquire::error::InquireError::OperationCanceled
            | inquire::error::InquireError::OperationInterrupted,
        ) => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    match source {
        RequestSource::Existing => {
            prompt_custom_type("RequestMessage JSON source (inline JSON, @FILE, or @-)")
        }
        RequestSource::Build => {
            Ok(
                request_builder::prompt_request_message()?.map(|message| RequestMessageInput {
                    source: serde_json::to_string(&message)
                        .expect("RequestMessage JSON serialization cannot fail"),
                    message,
                }),
            )
        }
    }
}

fn prompt_secret(message: &str) -> Result<Option<SecretInput>> {
    match inquire::Password::new(message)
        .without_confirmation()
        .prompt()
    {
        Ok(value) if value.is_empty() => Err(eyre!("secret input cannot be empty")),
        Ok(value) => Ok(Some(SecretInput { secret: value })),
        Err(
            inquire::error::InquireError::OperationCanceled
            | inquire::error::InquireError::OperationInterrupted,
        ) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

#[derive(Clone)]
struct RequestMessageInput {
    source: String,
    message: RequestMessage,
}

impl fmt::Debug for RequestMessageInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RequestMessageInput")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for RequestMessageInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.source)
    }
}

impl FromStr for RequestMessageInput {
    type Err = String;

    fn from_str(source: &str) -> std::result::Result<Self, Self::Err> {
        let message = read_request_message(source).map_err(|error| format!("{error:#}"))?;
        Ok(Self {
            source: source.to_owned(),
            message,
        })
    }
}

impl interactive_clap::ToCli for RequestMessageInput {
    type CliVariant = Self;
}

#[derive(Debug, Clone)]
struct SignerAccountInput {
    account_id: AccountId,
}

impl fmt::Display for SignerAccountInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.account_id.fmt(formatter)
    }
}

impl FromStr for SignerAccountInput {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        value
            .parse()
            .map(|account_id| Self { account_id })
            .map_err(|error| format!("invalid NEAR account ID: {error}"))
    }
}

impl interactive_clap::ToCli for SignerAccountInput {
    type CliVariant = Self;
}

#[derive(Debug, Clone)]
struct ExpectedPublicKeyInput {
    public_key: Ed25519PublicKey,
}

impl fmt::Display for ExpectedPublicKeyInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.public_key.fmt(formatter)
    }
}

impl FromStr for ExpectedPublicKeyInput {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        value
            .parse()
            .map(|public_key| Self { public_key })
            .map_err(|error| format!("invalid Ed25519 public key: {error}"))
    }
}

impl interactive_clap::ToCli for ExpectedPublicKeyInput {
    type CliVariant = Self;
}

#[derive(Debug, Clone)]
struct PathInput {
    path: PathBuf,
}

impl fmt::Display for PathInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.display().fmt(formatter)
    }
}

impl FromStr for PathInput {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if value.is_empty() {
            return Err("path cannot be empty".to_owned());
        }
        Ok(Self {
            path: PathBuf::from(value),
        })
    }
}

impl interactive_clap::ToCli for PathInput {
    type CliVariant = Self;
}

#[derive(Clone)]
struct SecretInput {
    secret: String,
}

impl fmt::Debug for SecretInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretInput([REDACTED])")
    }
}

impl fmt::Display for SecretInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.secret)
    }
}

impl FromStr for SecretInput {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if value.is_empty() {
            return Err("secret input cannot be empty".to_owned());
        }
        Ok(Self {
            secret: value.to_owned(),
        })
    }
}

impl interactive_clap::ToCli for SecretInput {
    type CliVariant = Self;
}

#[derive(Clone)]
struct ShellArgumentsInput {
    arguments: Vec<String>,
}

impl fmt::Debug for ShellArgumentsInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ShellArgumentsInput([REDACTED])")
    }
}

impl fmt::Display for ShellArgumentsInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&shell_words::join(&self.arguments))
    }
}

impl FromStr for ShellArgumentsInput {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let arguments = shell_words::split(value).map_err(|error| error.to_string())?;
        if arguments.is_empty() {
            return Err("near-cli signing tail cannot be empty".to_owned());
        }
        Ok(Self { arguments })
    }
}

impl interactive_clap::ToCli for ShellArgumentsInput {
    type CliVariant = Self;
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = WalletCli::parse();
    match <WalletCli as interactive_clap::FromCli>::from_cli(Some(cli), ()) {
        ResultFromCli::Ok(_) | ResultFromCli::Cancel(Some(_)) => Ok(()),
        ResultFromCli::Cancel(None) => {
            eprintln!("Goodbye!");
            Ok(())
        }
        ResultFromCli::Back => unreachable!("the top-level action disables back navigation"),
        ResultFromCli::Err(_, error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUEST: &str = r#"{"chain_id":"mainnet","signer_id":"wallet.near","nonce":42,"created_at":"2026-08-08T10:00:00Z","timeout_secs":3600,"request":{}}"#;

    #[test]
    fn parses_noninteractive_payload_command() {
        WalletCli::try_parse_from(["near-wallet", "payload", REQUEST, "--pretty"]).unwrap();
    }

    #[test]
    fn parses_noninteractive_keychain_sign_command() {
        WalletCli::try_parse_from([
            "near-wallet",
            "sign",
            REQUEST,
            "--sign-as",
            "alice.near",
            "--expected-public-key",
            "ed25519:11111111111111111111111111111111",
            "sign-with-keychain",
            "mainnet",
        ])
        .unwrap();
    }

    #[test]
    fn parses_every_structured_signing_backend() {
        let tails: &[&[&str]] = &[
            &["sign-with-legacy-keychain", "testnet"],
            &["sign-with-ledger", "usb"],
            &[
                "sign-with-plaintext-private-key",
                "ed25519:private-key-placeholder",
            ],
            &["sign-with-access-key-file", "/tmp/access-key.json"],
            &[
                "sign-with-seed-phrase",
                "seed phrase placeholder",
                "--seed-phrase-hd-path",
                "m/44'/397'/1'",
            ],
            &["custom", "sign-with-keychain network-config custom-network"],
        ];

        for tail in tails {
            let mut args = vec!["near-wallet", "sign", REQUEST, "--sign-as", "alice.near"];
            args.extend(tail.iter().copied());
            WalletCli::try_parse_from(args).unwrap();
        }
    }

    #[test]
    fn custom_signing_arguments_are_shell_parsed_without_execution() {
        let input: ShellArgumentsInput = "sign-with-keychain network-config 'custom network'"
            .parse()
            .unwrap();
        assert_eq!(
            input.arguments,
            ["sign-with-keychain", "network-config", "custom network"]
        );
    }

    #[test]
    fn secret_debug_output_is_redacted() {
        let secret: SecretInput = "ed25519:a-secret".parse().unwrap();
        assert!(!format!("{secret:?}").contains("a-secret"));
    }
}
