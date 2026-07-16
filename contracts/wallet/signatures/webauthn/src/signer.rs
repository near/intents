use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use defuse_crypto::{Curve, Signer};
use defuse_digest::{Digest, sha2::Sha256};
use defuse_wallet::RequestMessage;
use defuse_wallet_sdk::{Proof, WalletSigner};
use defuse_webauthn::{ClientDataType, CollectedClientData, UserVerification, WebauthnAssertion};
use serde::Serialize;

use crate::{WalletWebauthn, WalletWebauthnAlgorithm, WalletWebauthnProof};

const ORIGIN: &str = "http://localhost";

#[derive(Debug, Clone, derive_more::AsRef)]
pub struct LocalSigner<S> {
    #[as_ref]
    signer: S,
    sign_count: Arc<AtomicU32>,
}

impl<S> LocalSigner<S> {
    #[inline]
    pub fn new(signer: S) -> Self {
        Self {
            signer,
            sign_count: Arc::new(AtomicU32::new(0)),
        }
    }

    #[inline]
    pub const fn signer(&self) -> &S {
        &self.signer
    }
}

impl<S> From<S> for LocalSigner<S> {
    #[inline]
    fn from(signer: S) -> Self {
        Self::new(signer)
    }
}

impl<A, UV, S> WalletSigner<WalletWebauthn<A, UV>> for LocalSigner<S>
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
        self.signer.public_key().into()
    }

    async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        let assertion = WebauthnAssertion {
            // https://w3c.github.io/webauthn/#table-authData
            authenticator_data: [
                Sha256::digest(ORIGIN).as_slice(), // rpIdHash
                &[0b00000001u8],                   // TODO: UV
                &self.sign_count.fetch_add(1, Ordering::SeqCst).to_be_bytes(),
            ]
            .concat(),
            client_data_json: serde_json::to_string(&CollectedClientData {
                typ: ClientDataType::Get,
                challenge: msg.hash().into(),
                origin: ORIGIN.to_string(),
            })
            .unwrap(),
        };

        // TODO: no .to_vec()
        let data = A::preprocess(assertion.prepare_payload()).as_ref().to_vec();

        Ok(serde_json::to_string(&WalletWebauthnProof::<A::Signature> {
            signature: self.signer.sign(&data).await?.into(),
            assertion,
        })
        .unwrap())
    }
}
