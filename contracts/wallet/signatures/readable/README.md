# Readable signing format

Library shared by the wallet-contract variants whose wallets sign
human-readable text. It renders an execution request or a NEP-641
authorization as text. A variant
rebuilds the text from the parsed message and its stored identity and verifies
the signature over exactly these bytes, so no text, summary or digest supplied
by a dApp is trusted. Each variant defines its identity fields and signature
envelope.

JSON fields that the contract does not define are ignored when a request is
parsed; they are neither shown nor executed.

## Format (v1)

A message is a prefix, warning lines (if any), a JSON document and a final LF.
It is ASCII and at most 16 KiB (16384 bytes).

**Prefix** of an execution request (each line ends with LF, then one empty
line):

```text
NEAR Wallet Contract: execute request (v1)
This signature authorizes NEAR wallet operations, not a login.
NEAR amounts and deposits are in yoctoNEAR; FT amounts are in token base units.
Gas is in gas units.
Function arguments and deployment state may be opaque: verify them before signing.
```

and of a NEP-641 authorization:

```text
NEAR Wallet Contract: offchain authorization (v1)
This signature authorizes the exact payload below, not a NEAR transaction.
The consuming application defines expiration and one-time-use requirements.
```

**Warnings** of an execution request follow its prefix, one per line, then one
empty line. For each request, its `add_extension` operations come first, then
its promises in order:

- `WARNING: add_extension gives <account> full control of <wallet>.`
- `WARNING: up to <amount> sent to <receiver> can be refunded to <refund_to>, not to <wallet>.`
  for a promise that attaches NEAR (transfers and deposits) and whose
  `refund_to` is another account than the executing wallet.

`<wallet>` is `this wallet`, or `wallet <account>` for a nested request.

**JSON** is indented with two spaces, with `": "` after keys. Every UTF-16
code unit from U+007F upward is escaped as lowercase `\uXXXX`. Arrays keep
their order, and fields have a fixed order:

- execution request: `chain_id`, `signer_id`, the identity fields, `nonce`,
  `created_at`, `timeout_secs`, `pay_for_gas`, `request`;
- NEP-641 authorization: `chain_id`, `signer_id`, the identity fields, `path`,
  `timestamp`, `payload`. The payload is an exact string and is never parsed;
- `request`: `internal` (wallet operations in their JSON form), then
  `external` (promises);
- promise: `receiver_id`, `refund_to` (`null` is the executing wallet),
  `actions`;
- action: `action`, then `amount` for `transfer`; `function_name`, `deposit`,
  `gas`, `gas_weight`, `args` for `function_call`; `deposit`, `state_init`
  for `deterministic_state_init`. Object keys of `state_init` are in
  JavaScript object order: integer keys (`"0"` to `"4294967294"`, without
  leading zeros) first in ascending order, then the others sorted as strings.

Values:

- Defaults are shown: `pay_for_gas` `false`, `refund_to` `null`, deposit
  `"0"`, `gas` `"0"`, `gas_weight` `"1"`.
- Amounts and deposits are full decimal integer strings, with no rounding,
  abbreviation or human-unit conversion. NEAR values are in yoctoNEAR, e.g.
  `"1500000000000000000000000"`. NEP-141 transfer amounts stay in the token's
  base units, e.g. `"1500000"`; the promise's `receiver_id` identifies the token.
  The prefix states these units; refund warnings append `yoctoNEAR`.
- `nonce` and `timeout_secs` are numbers; `gas` and `gas_weight`
  are decimal strings.
- Timestamps are RFC 3339 in UTC with `Z` and up to nine fractional digits
  without trailing zeros, e.g. `2026-11-03T12:34:56.12Z`. They must fit
  unsigned 64-bit Unix nanoseconds; the timeout is whole seconds fitting `u32`.

Function calls:

- `args` is shown once, as JSON in its original key order. The arguments must
  be exactly the compact serialization of that JSON, the same in
  `serde_json::to_vec` and `JSON.stringify`, so the text determines their
  bytes: no whitespace, duplicate keys, optional escapes or alternative number
  forms; integers within ±(2^53 - 1); no unpaired surrogates; integer keys
  (`"0"` to `"4294967294"`) first in ascending order. Empty and binary
  arguments are rejected; call a method without parameters with `{}`.
- NEP-141 `ft_transfer` and `ft_transfer_call` preserve `args.amount` as a
  canonical decimal `u128` string without leading zeros. Only the method's
  parameters are accepted (`receiver_id`, `amount`, `memo`, and `msg` for
  `ft_transfer_call`). The deposit must be exactly one yoctoNEAR. `receiver_id`
  must be a valid NEAR account ID; `ft_transfer_call` requires a string `msg`
  (which may be empty). Optional `memo` is a string or `null`. These checks
  reject malformed standard calls; they do not guarantee successful execution.
- A call of another wallet's `w_execute_extension` (this wallet as its
  extension) shows the nested `request` by the same rules, with its own
  warnings. The arguments must be exactly the serde form of that request (the
  contract's field order, defaults omitted).

Before rendering, raw function-call arguments (including base64 inside a
nested request), state data and 64 bytes per operation, promise and action
must fit 16 KiB in total. The combined UTF-8 length of the outer request's
method names is checked separately against 16 KiB. The final limit includes
the prefix, warnings and trailing LF. JSON arguments nest at most 127 levels.

Limits of the text:

- Strings inside `args` (such as `msg` of `ft_transfer_call`, or the inner
  arguments of a nested `w_execute_signed`) and state-init data are shown as
  they are, not decoded.
- Token amounts have no decimal point or symbol; the token is identified only
  by its account.
- Method names and text inside `args` come from the dApp. Only the prefix, the
  warning lines and the request structure are written by the contract.
- A shown call says nothing about the called contract's behavior.

## Verify control before funding

Deriving an account ID does not prove that anyone can sign for it. Before
showing a derived account as a deposit destination, have the user sign a
NEP-641 message with the selected variant and verify it. Bind it to the
network, the account and a fresh random value; it must not authorize anything
else. A connected address or public key alone is not enough.
