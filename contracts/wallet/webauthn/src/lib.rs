#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "p256")]
pub mod p256;

use ::core::marker::PhantomData;

use defuse_crypto::Curve;
pub use defuse_wallet_core as core;
use defuse_wallet_core::{RequestMessage, SignatureSchema};
pub use defuse_webauthn as webauthn;
use defuse_webauthn::{Algorithm, UserVerification, Webauthn, WebauthnPayload};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

// TODO: docs
pub struct WalletWebauthn<A: Algorithm, UV: UserVerification>(PhantomData<Webauthn<A, UV>>);

impl<A, UV> SignatureSchema for WalletWebauthn<A, UV>
where
    A: WalletWebauthnAlgorithm<Signature: DeserializeOwned>,
    UV: UserVerification,
    <A::Curve as Curve>::Signature: TryFrom<A::Signature>,
    for<'a> <A::Curve as Curve>::PublicKey: TryFrom<&'a A::PublicKey>,
{
    type PublicKey = A::PublicKey;

    fn verify(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
        let Ok(public_key) = <A::Curve as Curve>::PublicKey::try_from(public_key) else {
            return false;
        };

        let Ok(proof) = serde_json::from_str::<WalletWebauthnProof<A::Signature>>(proof) else {
            return false;
        };

        let Ok(signature) = <A::Curve as Curve>::Signature::try_from(proof.signature) else {
            return false;
        };

        // We hash the payload for webauthn, since:
        // 1. Authenticators are general-purpose signers and they usually
        //    implement blind singing.
        // 2. This reduces length of the `proof` submitted on-chain.
        Webauthn::<A, UV>::verify(&public_key, msg.hash(), &proof.payload, &signature)
    }
}

pub trait WalletWebauthnAlgorithm: Algorithm {
    type PublicKey;
    type Signature;
}

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletWebauthnProof<S> {
    #[serde(flatten)]
    pub payload: WebauthnPayload,

    pub signature: S,
}

// TODO
// #[cfg(test)]
// mod tests {
//     use std::time::Duration;

//     use defuse_wallet_core::Request;
//     use hex_literal::hex;
//     use rstest::rstest;

//     use super::*;

//     #[rstest]
//     #[case(
//         hex!("e2e9cb7ac57cb46d4da1ce1d1cc2c33bdfe17407c517916b522724a8ea2c6c50"),
//         RequestMessage {
//             chain_id: "mainnet".to_string(),
//             signer_id: "0scdb6cfeed476fc878af9d3246768cbe803714c87".parse().unwrap(),
//             nonce: todo!(),
//             created_at: "2026-07-02T14:17:35.756586Z".parse().unwrap(),
//             timeout: Duration::from_secs(3600),
//             request: Request::new(),
//         },
//         hex!("7cd68c54af557c3d5d7bb6810d90a3efd0eb09e11d13feae3df589d0a54e5629c56dd4e4f6ce48766fccd305135edcbfa1928b0e3131930825c464a68c7d6d0b")
//     )]
//     fn verify_ok(#[case] public_key: [u8; 32], #[case] msg: RequestMessage, #[case] proof: &str) {
//         assert!(
//             WalletEd25519::verify(Ed25519PublicKey(public_key), &msg, proof),
//             "signature is invalid"
//         );
//     }
// }
