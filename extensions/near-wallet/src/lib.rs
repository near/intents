use std::{
    ffi::{OsStr, OsString},
    io,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use defuse_crypto::ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature};
use defuse_nep413::{Nep413, Nep413Payload};
use defuse_wallet::RequestMessage;
use near_account_id::AccountId;
use serde::{Deserialize, Serialize};

/// JSON arguments accepted by `w_execute_signed` on a wallet contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecuteSignedArgs {
    pub msg: RequestMessage,
    pub proof: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NearSignedMessage {
    account_id: AccountId,
    public_key: Ed25519PublicKey,
    signature: Ed25519Signature,
}

/// Convert a wallet request message to the callback-less NEP-413 payload expected
/// by the wallet contract.
#[inline]
pub fn wallet_nep413_payload(msg: RequestMessage) -> Nep413Payload {
    msg.into_nep413_payload(None)
}

/// Read a request message from inline JSON, `@FILE`, or `@-` (stdin).
pub fn read_request_message(value: &str) -> Result<RequestMessage> {
    let json = match value {
        "@-" => {
            let mut json = String::new();
            io::Read::read_to_string(&mut io::stdin().lock(), &mut json)
                .context("failed to read request message from stdin")?;
            json
        }
        _ => {
            if let Some(path) = value.strip_prefix('@') {
                std::fs::read_to_string(path).with_context(|| {
                    format!(
                        "failed to read request message from {}",
                        Path::new(path).display()
                    )
                })?
            } else {
                value.to_owned()
            }
        }
    };

    serde_json::from_str(&json).context("invalid wallet RequestMessage JSON")
}

/// Construct the exact argument vector used to delegate NEP-413 signing to
/// `near-cli-rs`.
pub fn near_sign_args(
    payload: &Nep413Payload,
    signer_account_id: &AccountId,
    signing_args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> Result<Vec<OsString>> {
    ensure!(
        payload.callback_url.is_none(),
        "wallet-contract signatures do not support a NEP-413 callback URL"
    );
    let signing_args: Vec<_> = signing_args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
    ensure!(
        !signing_args.is_empty(),
        "a near-cli signing method is required"
    );

    let mut args = vec![
        "--quiet".into(),
        "message".into(),
        "sign-nep413".into(),
        "utf8".into(),
        payload.message.clone().into(),
        "nonce".into(),
        BASE64.encode(payload.nonce).into(),
        "recipient".into(),
        payload.recipient.clone().into(),
        "sign-as".into(),
        signer_account_id.as_str().into(),
    ];
    args.extend(signing_args);
    Ok(args)
}

/// Parse and verify the JSON emitted by `near message sign-nep413`.
///
/// In addition to checking the expected account and Ed25519 encodings, this
/// verifies the returned signature locally against the exact payload.
pub fn parse_near_signed_message(
    stdout: &[u8],
    signer_account_id: &AccountId,
    expected_public_key: Option<&Ed25519PublicKey>,
    payload: &Nep413Payload,
) -> Result<String> {
    let signed: NearSignedMessage =
        serde_json::from_slice(stdout).context("near-cli produced invalid signed-message JSON")?;

    ensure!(
        signed.account_id == *signer_account_id,
        "near-cli signed as '{}', expected '{}'",
        signed.account_id,
        signer_account_id,
    );
    if let Some(expected_public_key) = expected_public_key {
        ensure!(
            signed.public_key == *expected_public_key,
            "near-cli signed with '{}', expected '{}'",
            signed.public_key,
            expected_public_key,
        );
    }

    let public_key = signed
        .public_key
        .try_into()
        .context("near-cli returned an invalid Ed25519 public key")?;
    ensure!(
        Nep413::verify::<Ed25519>(&public_key, payload, &signed.signature.into()),
        "near-cli returned an invalid NEP-413 signature"
    );

    Ok(signed.signature.to_string())
}

/// Sign a wallet request through the installed `near-cli-rs` executable.
///
/// Child stdin and stderr are inherited so keychain/Ledger prompts remain
/// usable. Its stdout is captured, parsed, and never forwarded; callers can
/// therefore reserve their own stdout for machine-readable contract arguments.
pub fn sign_with_near(
    near_command: impl AsRef<OsStr>,
    payload: &Nep413Payload,
    signer_account_id: &AccountId,
    expected_public_key: Option<&Ed25519PublicKey>,
    signing_args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> Result<String> {
    let args = near_sign_args(payload, signer_account_id, signing_args)?;

    let output = Command::new(near_command)
        .args(args)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .output()
        .context("failed to run near-cli; install `near` or pass --near-command")?;

    if !output.status.success() {
        if let Some(code) = output.status.code() {
            bail!("near-cli signing failed with exit code {code}");
        }
        bail!("near-cli signing was terminated by a signal");
    }

    parse_near_signed_message(
        &output.stdout,
        signer_account_id,
        expected_public_key,
        payload,
    )
}

/// Build `w_execute_signed` arguments by delegating the signature to near-cli.
pub fn sign_request_with_near(
    near_command: impl AsRef<OsStr>,
    msg: RequestMessage,
    signer_account_id: &AccountId,
    expected_public_key: Option<&Ed25519PublicKey>,
    signing_args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> Result<ExecuteSignedArgs> {
    let payload = wallet_nep413_payload(msg.clone());
    let proof = sign_with_near(
        near_command,
        &payload,
        signer_account_id,
        expected_public_key,
        signing_args,
    )?;

    Ok(ExecuteSignedArgs { msg, proof })
}

/// Clap-compatible parser for a request-message JSON source.
pub fn parse_request_message_arg(value: &str) -> Result<RequestMessage, String> {
    read_request_message(value).map_err(|err| format!("{err:#}"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use defuse_wallet::Request;
    use hex_literal::hex;

    use super::*;

    fn request_message() -> RequestMessage {
        RequestMessage {
            pay_for_gas: false,
            chain_id: "mainnet".to_owned(),
            signer_id: "wallet.near".parse().unwrap(),
            nonce: 42,
            created_at: "2026-08-08T10:00:00Z".parse().unwrap(),
            timeout: Duration::from_hours(1),
            request: Request::new(),
        }
    }

    #[test]
    fn reads_inline_request_message() {
        let json = serde_json::to_string(&request_message()).unwrap();
        assert_eq!(read_request_message(&json).unwrap(), request_message());
    }

    #[test]
    fn builds_near_cli_arguments_without_a_shell() {
        let payload = Nep413Payload {
            message: "A message with spaces and 'quotes'".to_owned(),
            nonce: [7; 32],
            recipient: "mainnet @ wallet.near".to_owned(),
            callback_url: None,
        };
        let signer: AccountId = "alice.near".parse().unwrap();

        let args = near_sign_args(
            &payload,
            &signer,
            ["sign-with-keychain", "network-config", "mainnet"],
        )
        .unwrap();

        assert_eq!(
            args,
            [
                "--quiet",
                "message",
                "sign-nep413",
                "utf8",
                "A message with spaces and 'quotes'",
                "nonce",
                "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=",
                "recipient",
                "mainnet @ wallet.near",
                "sign-as",
                "alice.near",
                "sign-with-keychain",
                "network-config",
                "mainnet",
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn rejects_callback_url_for_near_cli_signing() {
        let payload = Nep413Payload::new("hello")
            .recipient("wallet.near")
            .callback_url("https://example.com/callback");
        let signer: AccountId = "alice.near".parse().unwrap();

        let err = near_sign_args(&payload, &signer, ["sign-with-keychain"]).unwrap_err();
        assert!(err.to_string().contains("callback URL"));
    }

    #[test]
    fn rejects_a_missing_signing_method() {
        let payload = Nep413Payload::new("hello").recipient("wallet.near");
        let signer: AccountId = "alice.near".parse().unwrap();

        let err = near_sign_args(&payload, &signer, std::iter::empty::<&str>()).unwrap_err();
        assert!(err.to_string().contains("signing method is required"));
    }

    #[test]
    fn parses_and_verifies_near_cli_output() {
        // NEP-413 vector from `defuse-nep413`.
        let payload = Nep413Payload {
            message: "Hello, world!".to_owned(),
            nonce: [0; 32],
            recipient: "intents.near".to_owned(),
            callback_url: None,
        };
        let signer: AccountId = "alice.near".parse().unwrap();
        let public_key = Ed25519PublicKey(hex!(
            "e2e9cb7ac57cb46d4da1ce1d1cc2c33bdfe17407c517916b522724a8ea2c6c50"
        ));
        let signature = Ed25519Signature(hex!(
            "e2ff6254871a3fec1853c167b42f0f14248c4cf7fef5452dc24d8dbdc5c4bf183ab707322b4d782d5f5a05571bae476c5f7ee41c473f3002e600865e46b75d0f"
        ));
        let stdout = serde_json::json!({
            "accountId": signer,
            "publicKey": public_key.to_string(),
            "signature": signature.to_string(),
        });

        assert_eq!(
            parse_near_signed_message(
                serde_json::to_vec_pretty(&stdout).unwrap().as_slice(),
                &"alice.near".parse().unwrap(),
                Some(&public_key),
                &payload,
            )
            .unwrap(),
            signature.to_string(),
        );
    }

    #[test]
    fn rejects_a_different_signer_account() {
        let payload = Nep413Payload::new("irrelevant");
        let stdout = br#"{
            "accountId": "mallory.near",
            "publicKey": "ed25519:11111111111111111111111111111111",
            "signature": "ed25519:1111111111111111111111111111111111111111111111111111111111111111"
        }"#;

        let err = parse_near_signed_message(stdout, &"alice.near".parse().unwrap(), None, &payload)
            .unwrap_err();
        assert!(err.to_string().contains("expected 'alice.near'"));
    }

    #[test]
    fn rejects_a_different_public_key() {
        let payload = Nep413Payload {
            message: "Hello, world!".to_owned(),
            nonce: [0; 32],
            recipient: "intents.near".to_owned(),
            callback_url: None,
        };
        let signer: AccountId = "alice.near".parse().unwrap();
        let public_key = Ed25519PublicKey(hex!(
            "e2e9cb7ac57cb46d4da1ce1d1cc2c33bdfe17407c517916b522724a8ea2c6c50"
        ));
        let signature = Ed25519Signature(hex!(
            "e2ff6254871a3fec1853c167b42f0f14248c4cf7fef5452dc24d8dbdc5c4bf183ab707322b4d782d5f5a05571bae476c5f7ee41c473f3002e600865e46b75d0f"
        ));
        let stdout = serde_json::to_vec(&serde_json::json!({
            "accountId": signer,
            "publicKey": public_key.to_string(),
            "signature": signature.to_string(),
        }))
        .unwrap();
        let expected = Ed25519PublicKey([9; 32]);

        let err = parse_near_signed_message(
            &stdout,
            &"alice.near".parse().unwrap(),
            Some(&expected),
            &payload,
        )
        .unwrap_err();
        assert!(err.to_string().contains("expected 'ed25519:"));
    }

    #[test]
    fn rejects_an_invalid_signature() {
        let payload = Nep413Payload {
            message: "Hello, world!".to_owned(),
            nonce: [0; 32],
            recipient: "intents.near".to_owned(),
            callback_url: None,
        };
        let public_key = Ed25519PublicKey(hex!(
            "e2e9cb7ac57cb46d4da1ce1d1cc2c33bdfe17407c517916b522724a8ea2c6c50"
        ));
        let stdout = serde_json::to_vec(&serde_json::json!({
            "accountId": "alice.near",
            "publicKey": public_key.to_string(),
            "signature": Ed25519Signature([0; 64]).to_string(),
        }))
        .unwrap();

        let err = parse_near_signed_message(
            &stdout,
            &"alice.near".parse().unwrap(),
            Some(&public_key),
            &payload,
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid NEP-413 signature"));
    }

    #[test]
    fn wallet_payload_has_no_callback() {
        let payload = wallet_nep413_payload(request_message());
        assert_eq!(payload.recipient, "mainnet @ wallet.near");
        assert!(payload.callback_url.is_none());
    }
}
