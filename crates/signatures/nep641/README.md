# NEP-641: Offchain Authorizations for Smart-Contracts

## Test vectors

[`vectors/offchain_message.json`](vectors/offchain_message.json) contains canonical
[`OffchainMessage`](src/message.rs) hashes for implementations in other languages to check
against. Every vector holds the message fields along with the expected, hex-encoded
`SHA3_256("NEAR_NEP641_OFFCHAIN_MESSAGE/V1" || borsh(message))` digest.

The vectors are asserted by this crate's test suite, so they cannot drift from the implementation.
