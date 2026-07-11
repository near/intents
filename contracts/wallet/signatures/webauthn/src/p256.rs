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

    use defuse_wallet::{
        AccountId, Gas, NearPromise, NearToken, Request, RequestMessage, SignatureSchema,
        actions::FunctionCall,
    };
    use defuse_webauthn::IgnoreUserVerification;
    use hex_literal::hex;
    use rstest::rstest;

    use crate::WalletWebauthn;

    use super::*;

    #[rstest]
    // https://nearblocks.io/txns/6vytw7NgAiPkJ3KYAyt18es4mDnwZ8knjpB7LHVJejAL
    #[case(
        hex!("03b87da10683d04e6ec4e2f1775556a63cbb01be843058eb737fe02f9e22663093"),
        RequestMessage {
            chain_id: "mainnet".to_string(),
            signer_id: "0se5eba21e8f191e1880e453794bc551dfa50a3419".parse().unwrap(),
            nonce: 2845491008,
            created_at: "2026-07-07T11:13:29Z".parse().unwrap(),
            timeout: Duration::from_hours(1),
            request: Request::new().external([
                NearPromise::new("v1.signer".parse::<AccountId>().unwrap())
                    .function_call(
                        FunctionCall::name("sign")
                            .args(hex!("7b2272657175657374223a7b227061796c6f61645f7632223a7b224563647361223a2230303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030227d2c22646f6d61696e5f6964223a302c2270617468223a22227d7d"))
                            .attach_deposit(NearToken::from_yoctonear(1))
                            .gas(Gas::from_gas(0))
                    )
            ]),
        },
        r#"{"authenticator_data":"SZYN5YgOjGh0NBcPZHZgW4_krrmihjLHmVzzuoMdl2MdAAAAAA==","client_data_json":"{\"type\":\"webauthn.get\",\"challenge\":\"BvJpGRQxNyM3oMYGoVgi40m9DV7DF3BPl77xpO1vXh0\",\"origin\":\"http://localhost:5173\",\"crossOrigin\":false}","signature":"p256:1AmAt2duibsS6ohWTPXwpb98JMMJu5gZEyCKxD2EktkYsApkaRRXyHNjcdpKMk428Dfy5DLMTg6uF3KmxRdPGyv"}"#
    )]
    fn verify_ok(
        #[case] public_key: impl Into<P256CompressedPublicKey>,
        #[case] msg: RequestMessage,
        #[case] proof: &str,
    ) {
        assert!(
            WalletWebauthn::<P256, IgnoreUserVerification>::verify(&public_key.into(), &msg, proof),
            "signature is invalid"
        );
    }
}
