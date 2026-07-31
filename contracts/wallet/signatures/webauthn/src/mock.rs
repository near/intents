use defuse_crypto::{Curve, Signer};
use defuse_wallet::{RequestMessage, offchain::OffchainMessage};
use defuse_wallet_sdk::{Proof, WalletSigner};
use defuse_webauthn::{UserVerification, mock::MockWebauthnSigner};
use impl_tools::autoimpl;
use serde::Serialize;

use crate::{WalletWebauthn, WalletWebauthnAlgorithm, WalletWebauthnProof};

/// Mock [`WalletSigner`] for [`WalletWebauthn`] signature schema
#[autoimpl(Debug, Clone where S: trait)]
pub struct MockWalletWebauthnSigner<
    A: WalletWebauthnAlgorithm,
    UV: UserVerification,
    S: Signer<A::Curve>,
>(MockWebauthnSigner<A, UV, S>);

impl<A, UV, S> MockWalletWebauthnSigner<A, UV, S>
where
    A: WalletWebauthnAlgorithm,
    UV: UserVerification,
    S: Signer<A::Curve>,
{
    #[inline]
    pub fn new(signer: S) -> Self {
        Self(MockWebauthnSigner::new(signer))
    }

    #[inline]
    pub const fn signer(&self) -> &S {
        self.0.signer()
    }

    async fn sign_hash(&self, hash: &[u8; 32]) -> Result<Proof, S::Error>
    where
        A::Signature: Serialize + From<<A::Curve as Curve>::Signature>,
    {
        let (assertion, signature) = self.0.sign(hash).await?;

        Ok(serde_json::to_string(&WalletWebauthnProof::<A::Signature> {
            signature: signature.into(),
            assertion,
        })
        .expect("JSON: failed to serialize"))
    }
}

impl<A, UV, S> WalletSigner<WalletWebauthn<A, UV>> for MockWalletWebauthnSigner<A, UV, S>
where
    A: WalletWebauthnAlgorithm,
    UV: UserVerification,
    A::PublicKey: From<<A::Curve as Curve>::PublicKey>,
    A::Signature: Serialize + From<<A::Curve as Curve>::Signature>,
    <A::Curve as Curve>::Signature: TryFrom<A::Signature>,
    for<'a> <A::Curve as Curve>::PublicKey: TryFrom<&'a A::PublicKey>,
    S: Signer<A::Curve>,
{
    type Error = S::Error;

    #[inline]
    fn public_key(&self) -> A::PublicKey {
        self.signer().public_key().into()
    }

    async fn sign_wallet_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        self.sign_hash(&msg.hash()).await
    }

    async fn sign_offchain_msg(&self, msg: &OffchainMessage) -> Result<Proof, Self::Error> {
        self.sign_hash(&msg.hash()).await
    }
}
