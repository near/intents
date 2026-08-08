use std::process::Command;

const REQUEST: &str = r#"{"chain_id":"mainnet","signer_id":"wallet.near","nonce":42,"created_at":"2026-08-08T10:00:00Z","timeout_secs":3600,"request":{}}"#;

fn near_wallet() -> Command {
    Command::new(env!("CARGO_BIN_EXE_near-wallet"))
}

#[test]
fn generated_help_lists_interactive_actions_and_backends() {
    let output = near_wallet().arg("--help").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("payload"));
    assert!(help.contains("sign"));

    let output = near_wallet().args(["sign", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("--sign-as"));
    assert!(help.contains("sign-with-keychain"));
    assert!(help.contains("custom"));
}

#[test]
fn fully_specified_payload_has_machine_readable_stdout() {
    let output = near_wallet().args(["payload", REQUEST]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");

    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["message"], "{}");
    assert_eq!(payload["recipient"], "mainnet @ wallet.near");
    assert!(payload.get("callback_url").is_none());
}
