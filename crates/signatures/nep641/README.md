# NEP-641: Offchain Authorizations for Smart-Contracts

## Test vectors

[`vectors/offchain_message.json`](vectors/offchain_message.json) contains canonical
[`OffchainMessage`](src/message.rs) hashes for implementations in other languages to check
against. Every vector holds the message fields along with the expected, hex-encoded
`SHA3_256("NEAR_NEP641_OFFCHAIN_MESSAGE/V1" || borsh(message))` digest.

[`vectors/access_key_authorization.json`](vectors/access_key_authorization.json) continues into
[`AccessKeyAuthorization`](src/access_keys.rs), in six groups:

| Group | Covers |
| ----- | ------ |
| `nep413_payloads` | NEP-413 payload construction, and the `SHA_256` prehash that is signed |
| `signed` | complete authorization blobs, on both curves, that MUST verify |
| `verify_cases` | blobs that parse but MUST NOT verify: curve mismatch, tampering, strictness |
| `parse_cases` | the JSON accept/reject boundary |
| `account_id_cases` | account ID validation, which happens on deserialization only |
| `implicit_accounts` | implicit account derivation on both curves |

Signatures were produced with `ed25519-dalek` and `k256` from fixed seeds, which are the crates
`defuse-crypto` verifies against rather than a second implementation.

`verify_cases.ed25519_torsion_public_key` is worth singling out: it is a public key carrying an
order-8 torsion component, signed honestly. Verification here is cofactorless and rejects it,
while a cofactored verifier — which most high-level ed25519 libraries expose by default —
accepts it. The key is not itself low order, so the small-order check does not catch it. An
implementation that reproduces every other vector can still disagree on this one.

The vectors are asserted by this crate's test suite, so they cannot drift from the implementation.
