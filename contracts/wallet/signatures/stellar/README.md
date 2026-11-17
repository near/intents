# Stellar wallet contract

NEAR wallet contract controlled by a **Stellar Ed25519 key**. The key signs
the [readable signing format](../../README.md#readable-signing-format-v1),
wrapped into one JSON document, with the wallet's `signMessage` API
([SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md)),
e.g. Freighter's. It is a separate variant from the digest-signing Ed25519 one
and uses neither Stellar transactions nor Soroban authorization entries.

## Signed message

The inspected Freighter source uses a JSON pretty-printer for JSON messages
and an unformatted block for other text (see [display evidence](#clear-signing)).
To target that rendering model, the readable text is signed as one JSON
document with a final LF:

- `about`: the lines of the prefix;
- `warnings`: the warning lines, `[]` if there are none;
- the fields of the readable format, unchanged.

```json
{
  "about": [
    "NEAR Wallet Contract: execute request (v1)",
    "This signature authorizes NEAR wallet operations, not a login.",
    "Gas is in gas units.",
    "Function arguments and deployment state may be opaque: verify them before signing."
  ],
  "warnings": [
    "WARNING: up to 1 NEAR sent to bob.near can be refunded to refund.near, not to this wallet."
  ],
  "chain_id": "mainnet",
  "signer_id": "wallet.near",
  "stellar_address": "GDLVVGABQKYQVN6VJP7NHSLEA45A5YLS6PNKMIZFV4BBU2HXA5IRVHUR",
  "nonce": 42,
  "created_at": "2026-11-03T12:34:56.123456789Z",
  "timeout_secs": 300,
  "pay_for_gas": false,
  "request": {
    "internal": [],
    "external": [
      {
        "receiver_id": "bob.near",
        "refund_to": "refund.near",
        "actions": [
          {
            "action": "transfer",
            "amount": "1 NEAR"
          }
        ]
      }
    ]
  }
}
```

For an accepted message, its signed bytes are exactly the UTF-8 encoding of
`JSON.stringify(JSON.parse(message), null, 2) + "\n"`. The final LF is signed;
this equality describes the serialization model, not proof of what a wallet
actually displays. Function arguments and nested `w_execute_extension`
requests are JSON objects in it, not strings. The adapter rejects these
inputs to keep the document within the modeled display constraints:

- text with DEL or non-ASCII characters (in function arguments, token symbols,
  the NEP-641 payload, `chain_id`): the readable format escapes them as
  `\uXXXX`, and the wallet would show the characters themselves, including
  bidirectional controls and look-alike letters;
- state-init data whose keys JavaScript would reorder (base64 keys that are
  array indexes, such as `"1234"`, must come first in ascending order);
- a document above 16 KiB (16384 bytes). It is longer than the readable text
  of the same message.

## Identity and proof

- The wallet state stores the 32-byte Ed25519 public key; `w_public_key()`
  returns `ed25519:<base58>`. Weak (small-order) keys never verify.
- The document has one identity field after `signer_id`: `stellar_address`,
  the `G...` account ID of the key.
- The wallet signs `SHA-256("Stellar Signed Message:\n" || message)` with
  Ed25519.
- The proof is the 64-byte signature in standard padded base64 (88
  characters), as `signMessage` returns it.
- Proofs of execution requests, of NEP-641 authorizations and of the
  digest-signing variant are not interchangeable.

Authorization proves possession of the stored Ed25519 key, not authority over
a Stellar ledger account. The verifier does not query Stellar master-key
weights, other signers, or multisignature thresholds. Disabling that master
key or changing those thresholds on Stellar therefore does **not** revoke its
control of the NEAR wallet: valid signatures from the stored key remain
accepted while the NEAR wallet permits signature authorization. Use the NEAR
wallet's extension and signature controls to manage that authority.

Only a `G...` public-key identity is represented; contract (`C...`) and muxed
(`M...`) addresses are not supported signing identities. Before showing the
derived account as a deposit destination,
[verify control](../../README.md#verify-control-before-funding) of the key.

## Clear signing

The contract rebuilds and verifies the document intended for the wallet
prompt. Whether a particular wallet displays it completely and understandably
requires a real-wallet check. What the document does not decode is listed with the
[format](../../README.md#readable-signing-format-v1): JSON inside a string
value (such as `msg` of `ft_transfer_call` or the NEP-641 payload) stays an
escaped string.

The rendering model was inspected in [Freighter's signing component at
revision c1a8b4099069784cd4660be80b7b7a3bed512f86](https://github.com/stellar/freighter/blob/c1a8b4099069784cd4660be80b7b7a3bed512f86/extension/src/popup/components/signMessage/index.tsx):
the source package version is 5.53.0 and its JSON renderer is
`react-json-pretty` 2.2.0. These identify the inspected source, not a tested
installed wallet. Tests reproduce the serialization model only. Display in a
real wallet, on hardware wallets and in other Stellar wallets is **unverified**.
