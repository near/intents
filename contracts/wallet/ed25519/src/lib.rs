// TODO
#[cfg(all(feature = "contract", any(feature = "abi", near, test)))]
mod contract;

use ::core::str::FromStr;

use defuse_crypto::{
    Curve,
    ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
};
pub use defuse_wallet_core as core;
use defuse_wallet_core::{RequestMessage, SignatureSchema};

// TODO: docs
pub struct WalletEd25519;

impl SignatureSchema for WalletEd25519 {
    type PublicKey = Ed25519PublicKey;

    fn verify(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
        let Ok(signature) = Ed25519Signature::from_str(proof) else {
            return false;
        };

        let Ok(public_key) = <Ed25519 as Curve>::PublicKey::try_from(public_key) else {
            return false;
        };

        Ed25519::verify(&public_key, &msg.hash(), &signature.into())
    }
}

#[cfg(feature = "signer")]
const _: () = {
    use ::core::convert::Infallible;

    use async_trait::async_trait;
    use defuse_crypto::ed25519::ed25519_dalek::{self, SigningKey};
    use defuse_wallet_sdk::{Proof, Signer};

    #[cfg_attr(not(target_family = "wasm"), async_trait)]
    #[cfg_attr(target_family = "wasm", async_trait(?Send))]
    impl Signer<WalletEd25519> for SigningKey {
        type Error = Infallible;

        fn public_key(&self) -> Ed25519PublicKey {
            self.verifying_key().into()
        }

        async fn sign(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
            let signature: Ed25519Signature = ed25519_dalek::Signer::sign(self, &msg.hash()).into();
            Ok(signature.to_string())
        }
    }
};

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
