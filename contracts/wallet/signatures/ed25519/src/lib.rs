use core::str::FromStr;

use defuse_crypto::{
    Curve,
    ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
};
use defuse_wallet::{RequestMessage, SignatureSchema};

/// Simple [`Ed25519`] wallet [signature schema](SignatureSchema)
/// over [canonical request hash](RequestMessage::hash).
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
    use core::convert::Infallible;

    use async_trait::async_trait;
    use defuse_crypto::ed25519::ed25519_dalek::{Signer, SigningKey};
    use defuse_wallet_sdk::{Proof, WalletSigner};

    #[cfg_attr(not(target_family = "wasm"), async_trait)]
    #[cfg_attr(target_family = "wasm", async_trait(?Send))]
    impl WalletSigner<WalletEd25519> for SigningKey {
        type Error = Infallible;

        #[inline]
        fn public_key(&self) -> Ed25519PublicKey {
            self.verifying_key().into()
        }

        async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
            let sig = self.sign(&msg.hash());

            Ok(Ed25519Signature::from(sig).to_string())
        }
    }
};

#[cfg(feature = "contract")]
const _: () = {
    use defuse_wallet::wallet;

    wallet! {
        #[wallet(
            schema = WalletEd25519,
            metadata(
                standard(standard = "wallet-ed25519", version = "1.0.0")
            )
        )]
        struct Contract(_);
    }
};

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use defuse_wallet::{
        AccountId, Gas, NearPromise, NearToken, Request, WalletOp, actions::FunctionCall,
    };
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    // https://nearblocks.io/txns/fpeDPPwee7iYsfLundSCMWmVJektdM9gZYC3sTmYMTU
    #[case(
        hex!("8565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466"),
        RequestMessage {
            chain_id: "mainnet".to_string(),
            signer_id: "0scdb6cfeed476fc878af9d3246768cbe803714c87".parse().unwrap(),
            nonce: 2169412064,
            created_at: "2026-07-02T14:17:35.756586Z".parse().unwrap(),
            timeout: Duration::from_hours(1),
            request: Request::new()
                .internal([
                    WalletOp::AddExtension {
                        account_id: "extension.near".parse().unwrap(),
                    },
                    WalletOp::RemoveExtension {
                        account_id: "extension.near".parse().unwrap()
                    },
                ])
                .external([
                    NearPromise::new("v1.signer".parse::<AccountId>().unwrap())
                        .function_call(
                            FunctionCall::name("sign")
                                .args(hex!("7b2272657175657374223a7b22646f6d61696e5f6964223a302c2270617468223a22222c227061796c6f61645f7632223a7b224563647361223a2230313238666462613032363931383433303639616261373063303532336239633433663462306465346533343936323436326230353235343930373830613533227d7d7d"))
                                .attach_deposit(NearToken::from_yoctonear(1))
                                .gas(Gas::from_tgas(30))
                        )
                ]),
        },
        hex!("e4822e15e5988bf08c80b72f2d1292b7229029f342d42bb9dfe4e230c66c10a6c4a86a47ddc58b1446baedf2f1312294d59638c812082a0124e513d4eb16c40e")
    )]
    fn verify_ok(
        #[case] public_key: impl Into<Ed25519PublicKey>,
        #[case] msg: RequestMessage,
        #[case] proof: impl Into<Ed25519Signature>,
    ) {
        assert!(
            WalletEd25519::verify(&public_key.into(), &msg, &proof.into().to_string()),
            "signature is invalid"
        );
    }
}
