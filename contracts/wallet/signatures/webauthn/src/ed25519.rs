pub use defuse_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
pub use defuse_webauthn::ed25519::Ed25519;

use crate::WalletWebauthnAlgorithm;

impl WalletWebauthnAlgorithm for Ed25519 {
    type PublicKey = Ed25519PublicKey;
    type Signature = Ed25519Signature;
}

#[cfg(test)]
mod tests {
    use defuse_crypto::ed25519::ed25519_dalek;
    use defuse_wallet::{DEFAULT_TIMEOUT, Request, RequestMessage, SignatureSchema, Timestamp};
    use defuse_wallet_sdk::{MAINNET, WalletSigner};
    use defuse_webauthn::IgnoreUserVerification;
    use rand::rng;

    use crate::{WalletWebauthn, mock::MockWalletWebauthnSigner};

    use super::*;

    #[tokio::test]
    async fn sign_verify_ok() {
        type SS = WalletWebauthn<Ed25519, IgnoreUserVerification>;

        let signer = MockWalletWebauthnSigner::new(ed25519_dalek::SigningKey::generate(&mut rng()));

        let msg = RequestMessage {
            external: false,
            chain_id: MAINNET.to_string(),
            signer_id: "signer.near".parse().unwrap(),
            nonce: 0,
            created_at: Timestamp::now(),
            timeout: DEFAULT_TIMEOUT,
            request: Request::new(),
        };

        let proof = WalletSigner::<SS>::sign_request_msg(&signer, &msg)
            .await
            .unwrap();

        assert!(
            SS::verify(&WalletSigner::<SS>::public_key(&signer), &msg, &proof),
            "signer produced invalid signature"
        );
    }
}
