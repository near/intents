#[cfg(feature = "contract")]
mod contract;
#[cfg(feature = "signer")]
mod signer;
#[cfg(feature = "signer")]
pub use self::signer::*;

use core::str::FromStr;

pub use defuse_crypto as crypto;
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
    #[case(
        hex!("42be545513aacf13e895cae91e4ecd04498d7e9556795855c8787dbdccf55e9c"),
        RequestMessage {
            pay_for_gas: false,
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
        hex!("3dedecfbde9e59cebdc7f9bcca1de1fd52bcc2fa0512ea41d7f12e430bbf2dc58b70c5fc8ceeadd21d70540172b977b1f631776102ea79f858cc039c6ef0870e")
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
