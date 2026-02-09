use defuse_crypto::{
    Ed25519PublicKey, Ed25519Signature, P256Signature, Payload, PublicKey, Signature,
    SignedPayload, compress_public_key,
};
use defuse_webauthn::{Algorithm, Ed25519, P256, PayloadSignature, UserVerification};
use near_sdk::{CryptoHash, env, near, serde::de::DeserializeOwned, serde_json};

use super::{DefusePayload, ExtractDefusePayload};

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SignedWebAuthnPayload {
    pub payload: String,
    pub public_key: PublicKey,
    // schemars@0.8 does not respect it's `schemars(bound = "...")`
    // attribute: https://github.com/GREsau/schemars/blob/104b0fd65055d4b46f8dcbe38cdd2ef2c4098fe2/schemars_derive/src/lib.rs#L193-L206
    #[cfg_attr(all(feature = "abi", not(target_arch = "wasm32")), schemars(skip))]
    #[serde(flatten)]
    pub signature: PayloadSignature<Ed25519OrP256>,
}

impl Payload for SignedWebAuthnPayload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        env::sha256_array(self.payload.as_bytes())
    }
}

// TODO: rename
#[derive(Debug, Clone)]
pub struct Ed25519OrP256;

impl Algorithm for Ed25519OrP256 {
    type PublicKey = PublicKey;

    type Signature = Signature;

    #[inline]
    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool {
        match (public_key, signature) {
            (PublicKey::Ed25519(public_key), Signature::Ed25519(signature)) => Ed25519::verify(
                msg,
                &Ed25519PublicKey(*public_key),
                &Ed25519Signature(*signature),
            ),

            (PublicKey::P256(public_key), Signature::P256(signature)) => P256::verify(
                msg,
                &compress_public_key(*public_key),
                &P256Signature(*signature),
            ),
            _ => false,
        }
    }
}

impl SignedPayload for SignedWebAuthnPayload {
    type PublicKey = PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        self.signature
            .verify(self.hash(), &self.public_key, UserVerification::Ignore)
            .then_some(&self.public_key)
            .copied()
    }
}

impl<T> ExtractDefusePayload<T> for SignedWebAuthnPayload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::{AccountIdRef, serde_json};

    #[test]
    fn p256() {
        let p: SignedWebAuthnPayload = serde_json::from_str(r#"{
  "standard": "webauthn",
  "payload": "{\"signer_id\":\"0x3602b546589a8fcafdce7fad64a46f91db0e4d50\",\"verifying_contract\":\"defuse.test.near\",\"deadline\":\"2025-03-30T00:00:00Z\",\"nonce\":\"A3nsY1GMVjzyXL3mUzOOP3KT+5a0Ruy+QDNWPhchnxM=\",\"intents\":[{\"intent\":\"transfer\",\"receiver_id\":\"user1.test.near\",\"tokens\":{\"nep141:ft1.poa-factory.test.near\":\"1000\"}}]}",
  "public_key": "p256:2V8Np9vGqLiwVZ8qmMmpkxU7CTRqje4WtwFeLimSwuuyF1rddQK5fELiMgxUnYbVjbZHCNnGc6fAe4JeDcVxgj3Q",
  "signature": "p256:3KBMZ72BHUiVfE1ey5dpi3KgbXvSEf9kuxgBEax7qLBQtidZExxxjjQk1hTTGFRrPvUoEStfrjoFNVVW4Abar94W",
  "client_data_json": "{\"type\":\"webauthn.get\",\"challenge\":\"4cveZsIe6p-WaEcL-Lhtzt3SZuXbYsjDdlFhLNrSjjk\",\"origin\":\"https://defuse-widget-git-feat-passkeys-defuse-94bbc1b2.vercel.app\"}",
  "authenticator_data": "933cQogpBzE3RSAYSAkfWoNEcBd3X84PxE8iRrRVxMgdAAAAAA=="
}"#).unwrap();

        let public_key = p.verify().expect("invalid signature");
        assert_eq!(
            public_key,
            "p256:2V8Np9vGqLiwVZ8qmMmpkxU7CTRqje4WtwFeLimSwuuyF1rddQK5fELiMgxUnYbVjbZHCNnGc6fAe4JeDcVxgj3Q"
                .parse()
                .unwrap(),
        );
        assert_eq!(
            public_key.to_implicit_account_id(),
            AccountIdRef::new_or_panic("0x3602b546589a8fcafdce7fad64a46f91db0e4d50")
        );
    }

    #[test]
    fn ed25519() {
        let p: SignedWebAuthnPayload = serde_json::from_str(r#" {
  "standard": "webauthn",
  "payload": "{\"signer_id\":\"19a8cd22b37802c3cbc0031f55c70f3858ac48dbfb7697c435da637fea0e0e47\",\"verifying_contract\":\"intents.near\",\"deadline\":{\"timestamp\":1732035219},\"nonce\":\"XVoKfmScb3G+XqH9ke/fSlJ/3xO59sNhCxhpG821BH8=\",\"intents\":[{\"intent\":\"token_diff\",\"diff\":{\"nep141:base-0x833589fcd6edb6e08f4c7c32d4f71b54bda02913.omft.near\":\"-1000\",\"nep141:eth-0xdac17f958d2ee523a2206206994597c13d831ec7.omft.near\":\"998\"}}]}",
  "public_key": "ed25519:2jAUugnvWPvMaftKj5TDkyfsfxBwYjkMSf5MRtqDUMHY",
  "signature": "ed25519:2yBp5oExa9BBZQf8habpjLUaSiprvT7srHrK38Bxt9zL1yrkQSeeXMLmkihKCd9frmTdk24YctUdzNN5nGqHWHgb",
  "client_data_json": "{\"type\":\"webauthn.get\",\"challenge\":\"PfRFOFrLxCfyomuDryxhv6v2OzJIWqyMXaMikUYHSmY\",\"origin\":\"http://localhost:3000\"}",
  "authenticator_data": "SZYN5YgOjGh0NBcPZHZgW4_krrmihjLHmVzzuoMdl2MFZ50DuA"
}"#).unwrap();

        let public_key = p.verify().expect("invalid signature");
        assert_eq!(
            public_key,
            "ed25519:2jAUugnvWPvMaftKj5TDkyfsfxBwYjkMSf5MRtqDUMHY"
                .parse()
                .unwrap(),
        );
        assert_eq!(
            public_key.to_implicit_account_id(),
            AccountIdRef::new_or_panic(
                "19a8cd22b37802c3cbc0031f55c70f3858ac48dbfb7697c435da637fea0e0e47"
            )
        );
    }
}
