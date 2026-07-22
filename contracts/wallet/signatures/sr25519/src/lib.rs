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
    sr25519::{Sr25519, Sr25519PublicKey, Sr25519Signature},
};
use defuse_wallet::{RequestMessage, SignatureSchema};

/// [`Sr25519`] (Schnorr on Ristretto255) wallet [signature schema](SignatureSchema)
/// over [canonical request hash](RequestMessage::hash), wrapped in
/// `<Bytes>...</Bytes>` before signing.
///
/// Polkadot/Substrate wallets (Polkadot.js, Talisman, Subwallet, …) wrap
/// arbitrary messages in `<Bytes>...</Bytes>` when using `signRaw`. This
/// schema mirrors that wrap during verification so users can sign the
/// 32-byte request hash produced by [`RequestMessage::hash`] with any
/// standard Substrate wallet.
pub struct WalletSr25519;

impl WalletSr25519 {
    /// Wraps `msg` in `<Bytes>...</Bytes>` as done by Polkadot.js Extension
    /// (and other Substrate wallets) on `signRaw`.
    #[inline]
    pub(crate) fn signed_message(msg: &[u8]) -> Vec<u8> {
        [b"<Bytes>", msg, b"</Bytes>"].concat()
    }
}

impl SignatureSchema for WalletSr25519 {
    type PublicKey = Sr25519PublicKey;

    fn verify(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
        let Ok(signature) = Sr25519Signature::from_str(proof) else {
            return false;
        };

        let Ok(signature) = <Sr25519 as Curve>::Signature::try_from(signature) else {
            return false;
        };

        let Ok(public_key) = <Sr25519 as Curve>::PublicKey::try_from(public_key) else {
            return false;
        };

        Sr25519::verify(&public_key, &Self::signed_message(&msg.hash()), &signature)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use defuse_wallet::{Request, RequestMessage, Timestamp};
    use defuse_wallet_sdk::WalletSigner;
    use schnorrkel::Keypair;

    use super::*;

    /// A signature over the wrong message must be rejected.
    #[tokio::test]
    async fn wrong_message_rejected() {
        let signer = WalletSr25519Signer(Keypair::generate());

        let msg1 = RequestMessage {
            chain_id: "mainnet".to_string(),
            signer_id: "0s0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            nonce: 0,
            created_at: Timestamp::UNIX_EPOCH,
            timeout: Duration::from_hours(1),
            request: Request::new(),
        };
        let mut msg2 = msg1.clone();
        msg2.nonce = 1;

        let proof = signer.sign_wallet_msg(&msg1).await.unwrap();

        assert!(
            !WalletSr25519::verify(&signer.public_key(), &msg2, &proof),
            "signature over different message passed verification",
        );
    }
}
