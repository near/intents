use std::{
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use defuse_crypto::{Curve, Signer};
use hex_literal::hex;

use crate::{Algorithm, ClientDataType, CollectedClientData, UserVerification, WebauthnAssertion};

/// Mock origin
const ORIGIN: &str = "http://localhost";

/// SHA-256 hash of [`ORIGIN`]'s Relaying Party ID
///
/// See <https://w3c.github.io/webauthn/#rp-id>
const RP_ID_HASH: [u8; 32] =
    hex!("49960de5880e8c687434170f6476605b8fe4aeb9a28632c7995cf3ba831d9763");

/// Mock signer for [`Webauthn`](crate::Webauthn) signature schema
#[derive(Debug, Clone)]
pub struct MockWebauthnSigner<A: Algorithm, UV: UserVerification, S: Signer<A::Curve>> {
    sign_count: Arc<AtomicU32>,
    signer: S,
    _algorithm: PhantomData<fn() -> A>,
    _user_verification: PhantomData<fn() -> UV>,
}

impl<A, UV, S> MockWebauthnSigner<A, UV, S>
where
    A: Algorithm,
    UV: UserVerification,
    S: Signer<A::Curve>,
{
    #[inline]
    pub fn new(signer: S) -> Self {
        Self {
            sign_count: Arc::new(AtomicU32::new(0)),
            signer,
            _algorithm: PhantomData,
            _user_verification: PhantomData,
        }
    }

    #[inline]
    pub const fn signer(&self) -> &S {
        &self.signer
    }

    pub async fn sign(
        &self,
        challenge: impl Into<Vec<u8>>,
    ) -> Result<(WebauthnAssertion, <A::Curve as Curve>::Signature), S::Error> {
        let assertion = WebauthnAssertion {
            // https://w3c.github.io/webauthn/#table-authData
            authenticator_data: [
                // rpIdHash
                RP_ID_HASH.as_slice(),
                // flags
                &[{
                    let mut flags = 0b00000001u8; // UP (User Present)
                    if UV::REQUIRED {
                        flags |= 0b00000100u8; // set UV (User Verified)
                    }
                    flags
                }],
                // signCount
                &self.sign_count.fetch_add(1, Ordering::SeqCst).to_be_bytes(),
            ]
            .concat(),

            // https://w3c.github.io/webauthn/#dictdef-collectedclientdata
            client_data_json: serde_json::to_string(&CollectedClientData {
                typ: ClientDataType::Get,
                challenge: challenge.into(),
                origin: ORIGIN.to_string(),
            })
            .expect("JSON: failed to serialize"),
        };

        let data = A::maybe_prehash(assertion.effective_msg())
            .as_ref()
            .to_vec();

        let signature = self.signer.sign(&data).await?;
        Ok((assertion, signature))
    }
}
