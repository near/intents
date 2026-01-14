use defuse_crypto::serde::AsCurve;
use defuse_crypto::{Curve, Payload, Secp256k1, SignedPayload};
use near_sdk::base64::engine::{Engine, general_purpose::STANDARD};
use near_sdk::serde_json::json;
use near_sdk::{CryptoHash, env, near};

/// [ADR-36 Standard reference](https://github.com/cosmos/cosmos-sdk/blob/main/docs/architecture/adr-036-arbitrary-signature.md)
/// [Usage docs](https://docs.keplr.app/api/guide/sign-arbitrary#adr-36-signing-with-signamino)
#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub struct Adr36Payload {
    pub message: String,
    /// The Bech32 address of the account that will sign the message.
    /// Example: `cosmos1skjwj5whet0lpe65qaq4rpq03hjxlwd5c9m9s6`, where:
    ///     * `cosmos`: network prefix
    ///     * `1`: separator
    ///     * remainder: data + checksum
    pub signer: String,
}

impl Adr36Payload {
    #[inline]
    pub const fn new(message: String, signer: String) -> Self {
        Self { message, signer }
    }

    /// [Implementation reference](https://github.com/chainapsis/keplr-wallet/blob/59b2e18122dc2ec3b12d3005fec709e4bcc885f8/packages/cosmos/src/adr-36/amino.ts#L88)
    #[inline]
    pub fn prehash(&self) -> Vec<u8> {
        let json = json!({
            "account_number": "0",
            "chain_id": "",
            "fee": {
              "amount": [],
              "gas": "0",
            },
            "memo": "",
            "msgs": [
              {
                "type": "sign/MsgSignData",
                "value": {
                  "data": STANDARD.encode(self.message.as_bytes()),
                  "signer": self.signer,
                },
              },
            ],
            "sequence": "0",
        });
        json.to_string().into_bytes()
    }
}

impl Payload for Adr36Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        env::sha256_array(self.prehash())
    }
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SignedAdr36Payload {
    pub payload: Adr36Payload,

    #[serde_as(as = "AsCurve<Secp256k1>")]
    pub signature: <Secp256k1 as Curve>::Signature,
}

impl Payload for SignedAdr36Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedAdr36Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Secp256k1::verify(&self.signature, &self.payload.hash(), &())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Adr36Payload, SignedAdr36Payload};
    use defuse_crypto::{Payload, SignedPayload};
    use hex_literal::hex;
    use near_sdk::CryptoHash;

    /// note: copy-paste from `erc191/lib.rs`
    const fn fix_v_in_signature(mut sig: [u8; 65]) -> [u8; 65] {
        if *sig.last().unwrap() >= 27 {
            // Ethereum only uses uncompressed keys, with corresponding value v=27/28
            // https://bitcoin.stackexchange.com/a/38909/58790
            *sig.last_mut().unwrap() -= 27;
        }
        sig
    }

    /// goland cosmos-sdk [repro](https://gist.github.com/kuksag/eeb8ef3a77e6751d53db006b206925ab)
    const REFERENCE_MESSAGE: &str = "Hello, ADR-036!";
    const REFERENCE_SECP256K1_SIGNER: &str = "cosmos1mnyn7x24xj6vraxeeq56dfkxa009tvhgknhm04";
    const REFERENCE_SHA256_HASH_MESSAGE_HEX: CryptoHash =
        hex!("5ac8daed449a016684fd64bade7510b75ccd7c6eefa31b60a10eb577b37575e3");
    const REFERENCE_SIGNATURE: [u8; 65] = hex!(
        "043485aac9cd7de64da9548b72635c061e07b20063488c0d3affc3c843b33c0458f799ac6592b6260c5ce326be4996d95ee8cfbea4ae76b820f2a7a01ad3a5cc1b"
    );
    const WRONG_REFERENCE_SIGNATURE: [u8; 65] = hex!(
        "c6ada709bab5a03bdbeb7e53e54ff77afbfec9e4f7b3d1b588e24f09d6f5dc305fa7cdb36c78d9f0c31859879eb930d28c890bcf9e27944e7e8808b1a53c09661c"
    );
    const REFERENCE_PUBKEY: [u8; 64] = hex!(
        "4646ae5047316b4230d0086c8acec687f00b1cd9d1dc634f6cb358ac0a9a8ffffe77b4dd0a4bfb95851f3b7355c781dd60f8418fc8a65d14907aff47c903a559"
    );

    #[test]
    fn test_expected_sha256_hash() {
        let payload = Adr36Payload::new(
            REFERENCE_MESSAGE.to_string(),
            REFERENCE_SECP256K1_SIGNER.to_string(),
        );
        assert_eq!(payload.hash(), REFERENCE_SHA256_HASH_MESSAGE_HEX);
    }

    #[test]
    fn test_reference_signature_verification_works() {
        let payload = Adr36Payload::new(
            REFERENCE_MESSAGE.to_string(),
            REFERENCE_SECP256K1_SIGNER.to_string(),
        );
        let signature = fix_v_in_signature(REFERENCE_SIGNATURE);

        assert_eq!(
            SignedAdr36Payload { payload, signature }.verify(),
            Some(REFERENCE_PUBKEY)
        );
    }

    #[test]
    fn test_reference_signature_verification_fails() {
        let payload = Adr36Payload::new(
            REFERENCE_MESSAGE.to_string(),
            REFERENCE_SECP256K1_SIGNER.to_string(),
        );
        let signature = fix_v_in_signature(WRONG_REFERENCE_SIGNATURE);

        assert_ne!(
            SignedAdr36Payload { payload, signature }.verify(),
            Some(REFERENCE_PUBKEY)
        );
    }
}
