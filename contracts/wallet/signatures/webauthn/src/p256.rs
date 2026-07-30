pub use defuse_crypto::p256::{P256CompressedPublicKey, P256Signature};
pub use defuse_webauthn::p256::P256;

use crate::WalletWebauthnAlgorithm;

impl WalletWebauthnAlgorithm for P256 {
    type PublicKey = P256CompressedPublicKey;
    type Signature = P256Signature;
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use defuse_crypto::p256::p256::{ecdsa::SigningKey, elliptic_curve::Generate};
    use defuse_wallet::{
        AccountId, DEFAULT_TIMEOUT, Gas, NearPromise, NearToken, Request, RequestMessage,
        SignatureSchema, Timestamp, actions::FunctionCall,
    };
    use defuse_wallet_sdk::{MAINNET, WalletSigner};
    use defuse_webauthn::IgnoreUserVerification;
    use hex_literal::hex;
    use rand::rng;
    use rstest::rstest;

    use crate::{WalletWebauthn, mock::MockWalletWebauthnSigner};

    use super::*;

    #[rstest]
    #[case(
        hex!("02c0cad83d84b6bf228366971ddcb1738b9af862c64446a9feea85964743a5b390"),
        RequestMessage {
            pay_for_gas: false,
            chain_id: "mainnet".to_string(),
            signer_id: "0se5eba21e8f191e1880e453794bc551dfa50a3419".parse().unwrap(),
            nonce: 2845491008,
            created_at: "2026-07-07T11:13:29Z".parse().unwrap(),
            timeout: Duration::from_hours(1),
            request: NearPromise::new("v1.signer".parse::<AccountId>().unwrap())
                .function_call(
                    FunctionCall::name("sign")
                        .args(hex!("7b2272657175657374223a7b227061796c6f61645f7632223a7b224563647361223a2230303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030227d2c22646f6d61696e5f6964223a302c2270617468223a22227d7d"))
                        .attach_deposit(NearToken::from_yoctonear(1))
                        .gas(Gas::from_gas(0))
                )
                .into(),
        },
        r#"{"authenticator_data":"SZYN5YgOjGh0NBcPZHZgW4_krrmihjLHmVzzuoMdl2MdAAAAAA==","client_data_json":"{\"type\":\"webauthn.get\",\"challenge\":\"fIVgtROAVRZ2pU2sSdzB4AmvMCBVbN6OHiFyeC_9Z3U\",\"origin\":\"http://localhost:5173\",\"crossOrigin\":false}","signature":"p256:2S9WKwvKY7zgLaC9ihYDMQmbGtCvqqYy6RvvpgqRs7VXBR3GKXke4LKLNKyvtWRRB3dmU3awuCh2EYY7Sfk34qmh"}"#
    )]
    fn verify_ok(
        #[case] public_key: impl Into<P256CompressedPublicKey>,
        #[case] msg: RequestMessage,
        #[case] proof: &str,
    ) {
        assert!(
            WalletWebauthn::<P256, IgnoreUserVerification>::verify_request_msg(
                &public_key.into(),
                &msg,
                proof
            ),
            "signature is invalid"
        );
    }

    #[tokio::test]
    async fn sign_verify_ok() {
        type SS = WalletWebauthn<P256, IgnoreUserVerification>;

        let signer = MockWalletWebauthnSigner::new(SigningKey::generate_from_rng(&mut rng()));

        let msg = RequestMessage {
            pay_for_gas: false,
            chain_id: MAINNET.to_string(),
            signer_id: "signer.near".parse().unwrap(),
            nonce: 0,
            created_at: Timestamp::now(),
            timeout: DEFAULT_TIMEOUT,
            request: Request::new(),
        };

        let proof = WalletSigner::<SS>::sign_wallet_msg(&signer, &msg)
            .await
            .unwrap();

        assert!(
            SS::verify_request_msg(&WalletSigner::<SS>::public_key(&signer), &msg, &proof),
            "signer produced invalid signature"
        );
    }
}
