use core::fmt::Display;

use defuse_crypto::{Curve, Signer};
use defuse_nep413::{Nep413, Nep413Payload};
use defuse_wallet::{RequestMessage, offchain::OffchainMessage};
use defuse_wallet_sdk::{Proof, WalletSigner};

use crate::{WalletNep413, WalletNep413Curve};

/// Signer wrapper for the [`WalletNep413`] signature schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::From, derive_more::AsRef)]
pub struct WalletNep413Signer<S>(pub S);

impl<S> WalletNep413Signer<S> {
    async fn sign_payload<C>(&self, payload: &Nep413Payload) -> Result<Proof, S::Error>
    where
        C: WalletNep413Curve,
        C::Proof: Display + From<<C as Curve>::Signature>,
        S: Signer<C>,
    {
        let signature = self.0.sign(&Nep413::prehash(payload)).await?;

        Ok(C::Proof::from(signature).to_string())
    }
}

impl<C, S> WalletSigner<WalletNep413<C>> for WalletNep413Signer<S>
where
    C: WalletNep413Curve,
    C::StoredPublicKey: From<<C as Curve>::PublicKey>,
    C::Proof: Display + From<<C as Curve>::Signature>,
    <C as Curve>::Signature: TryFrom<C::Proof>,
    for<'a> <C as Curve>::PublicKey: TryFrom<&'a C::StoredPublicKey>,
    S: Signer<C>,
{
    type Error = S::Error;

    #[inline]
    fn public_key(&self) -> C::StoredPublicKey {
        self.0.public_key().into()
    }

    async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        self.sign_payload::<C>(&msg.clone().into_nep413_payload(None))
            .await
    }

    async fn sign_offchain_msg(&self, msg: &OffchainMessage) -> Result<Proof, Self::Error> {
        self.sign_payload::<C>(&msg.clone().into_nep413_payload(None))
            .await
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use defuse_crypto::ed25519::ed25519_dalek::SigningKey;
    use defuse_wallet::{Request, SignatureSchema, Timestamp};

    use crate::ed25519::Ed25519;

    use super::*;

    type Schema = WalletNep413<Ed25519>;

    #[tokio::test]
    async fn signs_verifiable_request_and_offchain_messages() {
        let signer = WalletNep413Signer(SigningKey::from_bytes(&[9; 32]));
        let public_key = WalletSigner::<Schema>::public_key(&signer);

        let request = RequestMessage {
            pay_for_gas: false,
            chain_id: "mainnet".to_string(),
            signer_id: "wallet.near".parse().unwrap(),
            nonce: 1,
            created_at: Timestamp::UNIX_EPOCH,
            timeout: Duration::from_hours(1),
            request: Request::new(),
        };
        let request_proof = WalletSigner::<Schema>::sign_request_msg(&signer, &request)
            .await
            .unwrap();
        assert!(Schema::verify_request_msg(
            &public_key,
            &request,
            &request_proof,
        ));

        let offchain = OffchainMessage {
            chain_id: "mainnet".to_string(),
            signer_id: "wallet.near".parse().unwrap(),
            path: Vec::new(),
            timestamp: Timestamp::UNIX_EPOCH,
            payload: "payload".to_string(),
        };
        let offchain_proof = WalletSigner::<Schema>::sign_offchain_msg(&signer, &offchain)
            .await
            .unwrap();
        assert!(Schema::verify_offchain_msg(
            &public_key,
            &offchain,
            &offchain_proof,
        ));
    }
}
