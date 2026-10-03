mod address;
#[cfg(feature = "contract")]
mod contract;
mod message;
#[cfg(feature = "signer")]
mod signer;
#[cfg(feature = "signer")]
pub use self::signer::*;

pub use self::{address::*, message::*};

pub use defuse_crypto as crypto;
pub use defuse_eip712 as eip712;

use defuse_crypto::secp256k1::{Secp256k1UncompressedPublicKey, k256};
use defuse_eip712::{Eip712, Eip712Domain};
use defuse_wallet::{RequestMessage, SignatureSchema, offchain::OffchainMessage};
use serde::de::DeserializeOwned;

/// [EIP-712](https://eips.ethereum.org/EIPS/eip-712) domain of all messages
/// signed for wallet contracts.
pub const DOMAIN: Eip712Domain<'static> = Eip712Domain {
    name: "NEAR Wallet Contract",
    version: "1",
};

/// [EIP-712](https://eips.ethereum.org/EIPS/eip-712) (`eth_signTypedData_v4`)
/// wallet [signature schema](SignatureSchema).
///
/// It allows any Ethereum wallet (e.g. `MetaMask`, `WalletConnect`, Ledger)
/// to be used as the sole key of a NEAR wallet-contract account.
///
/// # Identity: address, not public key
///
/// The contract stores the signer's [Ethereum address](EthAddress) in its
/// [state](defuse_wallet::State) and derives it from the public key recovered
/// from each proof, so the binding to the key is preserved.
///
/// Ethereum wallets hand out the address without any signing ceremony (e.g.
/// `eth_requestAccounts`), while the public key can only be recovered from a
/// signature. Since the deterministic NEAR `AccountId` commits to the initial
/// state, storing the address lets a client know which account it controls as
/// soon as the user connects — one roundtrip less. It also keeps 44 bytes
/// more of the [ZBA](https://github.com/near/NEPs/blob/master/neps/nep-0448.md)
/// budget free.
///
/// [`w_public_key()`](defuse_wallet::contract::Wallet::w_public_key) returns
/// this address as `0x<hex>`.
///
/// # Clear signing
///
/// The signer never signs an opaque digest: the typed data mirrors the
/// *contents* of the message being authorized
/// ([`Eip712RequestMessage`] for
/// [`w_execute_signed()`](defuse_wallet::contract::Wallet::w_execute_signed),
/// [`Eip712AuthMessage`] for
/// [`w_resolve_auth()`](defuse_wallet::offchain::AuthResolver::w_resolve_auth)),
/// so the wallet displays what is actually going to be executed or authorized.
///
/// The request contents are *unpacked* into the typed data: each
/// [wallet operation](Eip712WalletOp) and [promise](Eip712NearPromise) (with
/// its [actions](Eip712NearAction)) is a typed structure of its own, so the
/// wallet renders them structurally instead of one opaque JSON blob. Only the
/// leaf `payload`s remain JSON.
///
/// The contract does not trust that display: it checks every member of the
/// signed typed data against the message it is about to act upon (see
/// [`Eip712RequestMessage::matches()`] and [`Eip712AuthMessage::matches()`]),
/// so a proof whose typed data says anything other than the message is
/// rejected. Leaf `payload`s are compared *semantically*: JSON whitespace and
/// key order are free (e.g. pretty-print them for display), but any extra
/// field the contract would ignore invalidates the proof, so the wallet can
/// never display more than what gets executed.
///
/// # Proof
///
/// `proof` is a JSON-serialized [`SignedEip712`], i.e. the signed typed data
/// `message` along with a recoverable 65-byte secp256k1 `signature`
/// (`r ‖ s ‖ v`, where `v` is the recovery id `0`/`1`) formatted as
/// `secp256k1:<base58>`.
///
/// Note that Ethereum wallets return `v` as `27`/`28`, so clients MUST
/// normalize it by subtracting `27`.
pub struct WalletEip712;

impl WalletEip712 {
    /// Recover the public key that signed given [proof](SignedEip712) along
    /// with the typed data it committed to.
    fn recover<M>(proof: &str) -> Option<(M, Secp256k1UncompressedPublicKey)>
    where
        M: Eip712Message + DeserializeOwned,
    {
        let SignedEip712::<M> { message, signature } = serde_json::from_str(proof).ok()?;

        let (signature, recovery_id) =
            <(k256::ecdsa::Signature, k256::ecdsa::RecoveryId)>::try_from(&signature).ok()?;

        let public_key = Eip712::recover(
            &DOMAIN.separator(),
            &message.struct_hash(),
            &signature,
            recovery_id,
        )?;

        Some((message, public_key.into()))
    }
}

impl SignatureSchema for WalletEip712 {
    /// The [Ethereum address](EthAddress) of the signer, derived from the
    /// public key recovered from the proof.
    type PublicKey = EthAddress;

    fn verify_request_msg(address: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
        Self::recover::<Eip712RequestMessage>(proof).is_some_and(|(signed, recovered)| {
            EthAddress::from(recovered) == *address && signed.matches(msg)
        })
    }

    fn verify_offchain_msg(address: &Self::PublicKey, msg: &OffchainMessage, proof: &str) -> bool {
        Self::recover::<Eip712AuthMessage>(proof).is_some_and(|(signed, recovered)| {
            EthAddress::from(recovered) == *address && signed.matches(msg)
        })
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use defuse_wallet::{
        AccountId, NearPromise, NearToken, Request, Timestamp, WalletOp, offchain::OffchainMessage,
    };
    use defuse_wallet_sdk::WalletSigner;
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    fn signer() -> WalletEip712Signer<k256::ecdsa::SigningKey> {
        WalletEip712Signer(
            k256::ecdsa::SigningKey::from_bytes(
                &hex!("4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318").into(),
            )
            .unwrap(),
        )
    }

    fn address() -> EthAddress {
        WalletSigner::<WalletEip712>::public_key(&signer())
    }

    fn request_msg() -> RequestMessage {
        RequestMessage {
            pay_for_gas: false,
            chain_id: "mainnet".to_string(),
            signer_id: "0s0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            nonce: 42,
            created_at: Timestamp::UNIX_EPOCH,
            timeout: Duration::from_hours(1),
            request: Request::new()
                .internal([WalletOp::AddExtension {
                    account_id: "extension.near".parse::<AccountId>().unwrap(),
                }])
                .external([NearPromise::new("bob.near".parse::<AccountId>().unwrap())
                    .transfer(NearToken::from_near(1))]),
        }
    }

    fn offchain_msg() -> OffchainMessage {
        OffchainMessage {
            chain_id: "mainnet".to_string(),
            signer_id: "0s0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            path: vec!["master.near".parse().unwrap()],
            timestamp: Timestamp::UNIX_EPOCH,
            payload: "Login to example.app at 2026-07-16T00:00:00Z".to_string(),
        }
    }

    fn sign_request(msg: &RequestMessage) -> String {
        tokio_test::block_on(WalletSigner::sign_request_msg(&signer(), msg)).unwrap()
    }

    fn sign_offchain(msg: &OffchainMessage) -> String {
        tokio_test::block_on(WalletSigner::<WalletEip712>::sign_offchain_msg(
            &signer(),
            msg,
        ))
        .unwrap()
    }

    #[rstest]
    fn verify_request_ok() {
        let msg = request_msg();
        assert!(
            WalletEip712::verify_request_msg(&address(), &msg, &sign_request(&msg)),
            "signature is invalid",
        );
    }

    #[rstest]
    fn verify_offchain_ok() {
        let msg = offchain_msg();
        assert!(
            WalletEip712::verify_offchain_msg(&address(), &msg, &sign_offchain(&msg)),
            "signature is invalid",
        );
    }

    /// The typed data the signer sees MUST bind every field of the message
    /// the contract acts upon.
    #[rstest]
    #[case::pay_for_gas(|msg: &mut RequestMessage| msg.pay_for_gas = !msg.pay_for_gas)]
    #[case::chain_id(|msg: &mut RequestMessage| msg.chain_id = "testnet".to_string())]
    #[case::signer_id(|msg: &mut RequestMessage| {
        msg.signer_id = "0s1111111111111111111111111111111111111111".parse().unwrap();
    })]
    #[case::nonce(|msg: &mut RequestMessage| msg.nonce += 1)]
    #[case::created_at(|msg: &mut RequestMessage| {
        msg.created_at = msg.created_at.saturating_add_unsigned(Duration::from_secs(1));
    })]
    #[case::timeout(|msg: &mut RequestMessage| msg.timeout = Duration::from_mins(1))]
    #[case::internal(|msg: &mut RequestMessage| {
        msg.request.internal = vec![WalletOp::SetSignatureMode { enable: false }];
    })]
    #[case::external(|msg: &mut RequestMessage| {
        msg.request.external = vec![
            NearPromise::new("eve.near".parse::<AccountId>().unwrap())
                .transfer(NearToken::from_near(1)),
        ];
    })]
    #[case::external_receiver(|msg: &mut RequestMessage| {
        msg.request.external[0].receiver_id = "eve.near".parse().unwrap();
    })]
    #[case::external_refund_to(|msg: &mut RequestMessage| {
        msg.request.external[0].refund_to = Some("eve.near".parse().unwrap());
    })]
    fn verify_request_tampered(#[case] tamper: fn(&mut RequestMessage)) {
        let signed = request_msg();
        let proof = sign_request(&signed);

        let mut executed = signed;
        tamper(&mut executed);

        assert!(
            !WalletEip712::verify_request_msg(&address(), &executed, &proof),
            "proof verified against a message other than the signed one",
        );
    }

    #[rstest]
    #[case::chain_id(|msg: &mut OffchainMessage| msg.chain_id = "testnet".to_string())]
    #[case::signer_id(|msg: &mut OffchainMessage| {
        msg.signer_id = "0s1111111111111111111111111111111111111111".parse().unwrap();
    })]
    #[case::path_replaced(|msg: &mut OffchainMessage| {
        msg.path = vec!["eve.near".parse().unwrap()];
    })]
    #[case::path_extended(|msg: &mut OffchainMessage| {
        msg.path.push("v1.signer".parse().unwrap());
    })]
    #[case::path_top_level(|msg: &mut OffchainMessage| msg.path.clear())]
    #[case::timestamp(|msg: &mut OffchainMessage| {
        msg.timestamp = msg.timestamp.saturating_add_unsigned(Duration::from_secs(1));
    })]
    #[case::payload(|msg: &mut OffchainMessage| msg.payload = "something else".to_string())]
    fn verify_offchain_tampered(#[case] tamper: fn(&mut OffchainMessage)) {
        let signed = offchain_msg();
        let proof = sign_offchain(&signed);

        let mut resolved = signed;
        tamper(&mut resolved);

        assert!(
            !WalletEip712::verify_offchain_msg(&address(), &resolved, &proof),
            "proof verified against a message other than the signed one",
        );
    }

    /// Both message kinds live under the same EIP-712 domain, so they MUST be
    /// separated by their primary type.
    #[rstest]
    fn no_cross_type_replay() {
        assert!(
            !WalletEip712::verify_offchain_msg(
                &address(),
                &offchain_msg(),
                &sign_request(&request_msg()),
            ),
            "request proof was accepted as an authorization",
        );
        assert!(
            !WalletEip712::verify_request_msg(
                &address(),
                &request_msg(),
                &sign_offchain(&offchain_msg()),
            ),
            "authorization proof was accepted as a request",
        );
    }

    #[rstest]
    fn other_address() {
        let msg = request_msg();
        assert!(
            !WalletEip712::verify_request_msg(
                &EthAddress(hex!("d8da6bf26964af9d7eed9e03e53415d37aa96045")),
                &msg,
                &sign_request(&msg),
            ),
            "proof verified against another address",
        );
    }

    #[rstest]
    #[case::malformed("not-a-proof")]
    #[case::bare_signature(
        "secp256k1:7ND3TQwc9NkuqYQpjkR6cWJmDjndsDzZJukMcCwSVs4iWuMxCFJTsRoevHhSMwFHZoTrbn1wHu6mSQVWr4H7EJvV"
    )]
    #[case::other_curve(
        r#"{"message":{},"signature":"ed25519:5wHqR6FiZCotbRfPRskG8RzDRkmFmZy9Wweh4vBtZp8eTJ2TPq6rFY9oc6nZyVmEFEPxoxNwVF9pZd1JN84m7m5me"}"#
    )]
    fn malformed_proof(#[case] proof: &str) {
        assert!(
            !WalletEip712::verify_request_msg(&address(), &request_msg(), proof),
            "malformed proof passed verification",
        );
    }

    /// The NEAR `AccountId` of a wallet-contract instance is derived from its
    /// initial state, which holds the signer's *address* — so a client that
    /// only did `eth_requestAccounts` (no signing ceremony) can already tell
    /// which account it controls.
    #[rstest]
    fn account_id_derivable_from_address_alone() {
        use defuse_wallet_sdk::{GlobalContractId, State, StateInit, StateInitV1, Wallet};

        const CODE: GlobalContractId = GlobalContractId::CodeHash(hex!(
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));

        // what a client can compute knowing nothing but the `0x` address
        let derived = StateInit::V1(StateInitV1 {
            code: CODE,
            data: State::new(address()).as_storage(),
        })
        .derive_account_id();

        assert_eq!(
            derived,
            *Wallet::<WalletEip712>::new(CODE, signer()).account_id(),
        );
    }
}
