use defuse_crypto::{Payload, PublicKey, SignedPayload};
use defuse_erc191::SignedErc191Payload;
use defuse_nep413::SignedNep413Payload;
use defuse_sep53::SignedSep53Payload;
use defuse_tip191::SignedTip191Payload;
use defuse_ton_connect::SignedTonConnectPayload;
use derive_more::derive::From;
use near_sdk::{CryptoHash, near, serde::de::DeserializeOwned, serde_json};

use super::{
    DefusePayload, ExtractDefusePayload, raw::SignedRawEd25519Payload,
    webauthn::SignedWebAuthnPayload,
};

#[near(serializers = [json])]
#[serde(tag = "standard", rename_all = "snake_case")]
#[derive(Debug, Clone, From)]
/// Assuming wallets want to interact with Intents protocol, besides preparing the data in a certain
/// form, they have to have the capability to sign raw messages (off-chain signatures) using an algorithm we understand.
/// This enum solves that problem.
///
/// For example, because we support ERC-191 and know how to verify messages with that standard,
/// we can allow wallets, like Metamask, sign messages to perform intents without having to
/// support new cryptographic primitives and signing standards.
pub enum MultiPayload {
    /// NEP-413: The standard for message signing in Near Protocol.
    /// For more details, refer to [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md).
    Nep413(SignedNep413Payload),

    /// ERC-191: The standard for message signing in Ethereum, commonly used with `personal_sign()`.
    /// For more details, refer to [EIP-191](https://eips.ethereum.org/EIPS/eip-191).
    Erc191(SignedErc191Payload),

    /// TIP-191: The standard for message signing in Tron.
    /// For more details, refer to [TIP-191](https://github.com/tronprotocol/tips/blob/master/tip-191.md).
    Tip191(SignedTip191Payload),

    /// Raw Ed25519: The standard used by Solana Phantom wallets for message signing.
    /// For more details, refer to [Phantom Wallet's documentation](https://docs.phantom.com/solana/signing-a-message).
    RawEd25519(SignedRawEd25519Payload),

    /// WebAuthn: The standard for Passkeys.
    /// For more details, refer to [WebAuthn specification](https://w3c.github.io/webauthn/).
    #[serde(rename = "webauthn")]
    WebAuthn(SignedWebAuthnPayload),

    /// TonConnect: The standard for data signing in TON blockchain platform.
    /// For more details, refer to [TonConnect documentation](https://docs.tonconsole.com/academy/sign-data).
    TonConnect(SignedTonConnectPayload),

    /// SEP-53: The standard for signing data off-chain for Stellar accounts.
    /// See [SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md)
    Sep53(SignedSep53Payload),
}

impl Payload for MultiPayload {
    /// Hash of the envelope of the message.
    /// Note that different arms will yield different hash values,
    /// even if they include the same application-specific message in the envelope.
    /// For example, NEP-413, uses SHA-256, while ERC-191 uses Keccak256.
    #[inline]
    fn hash(&self) -> CryptoHash {
        match self {
            Self::Nep413(payload) => payload.hash(),
            Self::Erc191(payload) => payload.hash(),
            Self::Tip191(payload) => payload.hash(),
            Self::RawEd25519(payload) => payload.hash(),
            Self::WebAuthn(payload) => payload.hash(),
            Self::TonConnect(payload) => payload.hash(),
            Self::Sep53(payload) => payload.hash(),
        }
    }
}

impl SignedPayload for MultiPayload {
    type PublicKey = PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        match self {
            Self::Nep413(payload) => payload.verify().map(PublicKey::Ed25519),
            Self::Erc191(payload) => payload.verify().map(PublicKey::Secp256k1),
            Self::Tip191(payload) => payload.verify().map(PublicKey::Secp256k1),
            Self::RawEd25519(payload) => payload.verify().map(PublicKey::Ed25519),
            Self::WebAuthn(payload) => payload.verify(),
            Self::TonConnect(payload) => payload.verify().map(PublicKey::Ed25519),
            Self::Sep53(payload) => payload.verify().map(PublicKey::Ed25519),
        }
    }
}

impl<T> ExtractDefusePayload<T> for MultiPayload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        match self {
            Self::Nep413(payload) => payload.extract_defuse_payload(),
            Self::Erc191(payload) => payload.extract_defuse_payload(),
            Self::Tip191(payload) => payload.extract_defuse_payload(),
            Self::RawEd25519(payload) => payload.extract_defuse_payload(),
            Self::WebAuthn(payload) => payload.extract_defuse_payload(),
            Self::TonConnect(payload) => payload.extract_defuse_payload(),
            Self::Sep53(payload) => payload.extract_defuse_payload(),
        }
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::bs58;

    use super::*;

    #[test]
    fn raw_ed25519() {
        let p: MultiPayload = serde_json::from_str(r#"{"standard":"raw_ed25519","payload":"{\"signer_id\":\"74affa71ab030d400fdfa1bed033dfa6fd3ae34f92d17c046ebe368e80d53751\",\"verifying_contract\":\"intents.near\",\"deadline\":{\"timestamp\":1732035219},\"nonce\":\"XVoKfmScb3G+XqH9ke/fSlJ/3xO59sNhCxhpG821BH8=\",\"intents\":[{\"intent\":\"token_diff\",\"diff\":{\"nep141:base-0x833589fcd6edb6e08f4c7c32d4f71b54bda02913.omft.near\":\"-1000\",\"nep141:eth-0xdac17f958d2ee523a2206206994597c13d831ec7.omft.near\":\"998\"}}]}","public_key":"ed25519:8rVvtHWFr8hasdQGGD5WiQBTyr4iH2ruEPPVfj491RPN","signature":"ed25519:3vtbNQJHZfuV1s5DykzyjkbNLc583hnkrhTz57eDhd966iqzkor6Twgr4Loh2C195SCSEsiGfrd6KcxpjNq9ZbVj"}"#).unwrap();
        assert_eq!(
            bs58::encode(p.hash()).into_string(),
            "8LKE47o44ybZQR9ozLyDnvMDTh4Ao5ipy2mJWsYByG5Q"
        );
        assert_eq!(
            p.verify().unwrap(),
            "ed25519:8rVvtHWFr8hasdQGGD5WiQBTyr4iH2ruEPPVfj491RPN"
                .parse()
                .unwrap()
        );
    }
}
