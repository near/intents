mod contract;
mod convert;
mod ed25519;
mod secp256k1;

pub use self::convert::*;

pub use defuse_mpc_kdf as kdf;

use std::{borrow::Cow, cell::LazyCell, error::Error as StdError, fmt::Debug};

use defuse_mpc_kdf::{
    Additive, Derive, DeriveExt, DeriveSigner, RecoverableDeriveSigner, TweakSchema, crypto::Curve,
};
use defuse_near_promise::{AccountId, AccountIdRef, Gas, NearToken, actions::FunctionCall};
use defuse_near_sender::{ArcNearSender, NearSender, SentTransaction};
use impl_tools::autoimpl;
use near_kit::{CryptoHash, ExecutedOptimistic, ExecutionStatus, Near};

use crate::contract::{MpcContract, Payload, PublicKeyArgs, SignArgs, SignRequest, SignResponse};

/// MPC contract ID on Mainnet: `v1.signer`
pub const MAINNET_MPC_CONTRACT_ID: &AccountIdRef = AccountIdRef::new_or_panic("v1.signer");

/// On-chain MPC signer.
#[autoimpl(Clone where C::PublicKey: trait)]
pub struct MpcOnChainSigner<C: Curve> {
    sender: ArcNearSender,

    mpc_contract_id: AccountId,
    mpc_public_key: C::PublicKey,
    domain_id: u64,

    client: Near,
}

impl<C> MpcOnChainSigner<C>
where
    C: OnChainNearMpcCurve,
{
    pub async fn from_domain_id<S>(
        sender: S,
        mpc_contract_id: impl Into<AccountId>,
        domain_id: u64,
        client: Near,
    ) -> Result<Self, Error>
    where
        S: NearSender + 'static,
        S::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let mpc_contract_id = mpc_contract_id.into();

        let mpc_public_key = client
            .contract::<MpcContract>(&mpc_contract_id)
            .public_key(PublicKeyArgs { domain_id })
            .await?;

        Ok(Self {
            sender: sender.arced(),
            mpc_contract_id,
            mpc_public_key: C::extract_public_key(mpc_public_key)
                .ok_or(Error::InvalidDomain(domain_id))?,
            domain_id,
            client,
        })
    }

    /// Get predecessor ID on behalf of which messages will be singed.
    #[inline]
    pub fn predecessor_id(&self) -> Cow<'_, AccountIdRef> {
        self.sender.account_id()
    }

    async fn sign_extract<R>(
        &self,
        path: impl AsRef<str>,
        payload: Payload<'_>,
        extract: impl Fn(SignResponse) -> Option<R>,
    ) -> Result<R, Error>
    where
        C::PublicKey: Sync,
    {
        let sent = self.send_sign_request(path, payload).await?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            tx.hash = %CryptoHash::from(sent.tx_hash),
            tx.sender_id = %sent.sender_id,
            "{}::sign() transaction sent",
            self.mpc_contract_id,
        );

        let tx_outcome = sent
            .status(&self.client)
            // wait for `mpc_contract_id::sign()` receipt to execute
            .wait_until::<ExecutedOptimistic>()
            .await?;

        tx_outcome
            .receipts_outcome
            .into_iter()
            .filter(|o| o.outcome.executor_id == self.mpc_contract_id)
            .filter_map(|o| {
                let ExecutionStatus::SuccessValue(value) = o.outcome.status else {
                    return None;
                };
                serde_json::from_slice::<SignResponse>(&value).ok()
            })
            .find_map(extract)
            .ok_or_else(|| Error::SignatureNotFound(tx_outcome.transaction_outcome.id))
    }

    async fn send_sign_request(
        &self,
        path: impl AsRef<str>,
        payload: Payload<'_>,
    ) -> Result<SentTransaction, Error>
    where
        C::PublicKey: Sync,
    {
        const MPC_SIGN_GAS: Gas = Gas::from_tgas(15);

        self.sender
            .send(
                self.mpc_contract_id.clone(),
                vec![
                    FunctionCall::name("sign")
                        .attach_deposit(NearToken::from_yoctonear(1))
                        .gas(MPC_SIGN_GAS)
                        .args_json(SignArgs {
                            request: SignRequest {
                                path: path.as_ref().into(),
                                payload,
                                domain_id: self.domain_id,
                            },
                        })
                        .into(),
                ],
            )
            .await
            .map_err(Error::Sender)
    }
}

impl<C, P> DeriveSigner<C, P> for MpcOnChainSigner<C>
where
    C: OnChainNearMpcCurve<PublicKey: Clone + Send + Sync>,
    P: AsRef<str> + AsRef<[u8]>,
{
    type Error = Error;

    type Schema<'a>
        = Derive<Additive<C>, TweakSchema<C>>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        Additive::new(self.mpc_public_key.clone())
            .derive_with(defuse_mpc_kdf::tweak(self.predecessor_id()))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(
        mpc_contract_id = %self.mpc_contract_id,
        domain_id = self.domain_id,
    )))]
    async fn derive_sign(&self, path: P, msg: &[u8]) -> Result<C::Signature, Self::Error>
    where
        P: Send,
    {
        let path: &str = path.as_ref();
        self.sign_extract(
            path,
            C::msg_to_payload(msg).ok_or(Error::InvalidPayload)?,
            {
                let derived_public_key = LazyCell::new(|| self.derive_public_key(path));
                move |sig| {
                    let sig = C::extract_signature(sig)?;
                    if !C::verify(&derived_public_key, msg, &sig) {
                        // the signature is either invalid or not ours
                        return None;
                    }
                    Some(sig)
                }
            },
        )
        .await
    }
}

impl<C, P> RecoverableDeriveSigner<C, P> for MpcOnChainSigner<C>
where
    C: RecoverableOnChainNearMpcCurve<PublicKey: Clone + PartialEq + Send + Sync, RecoveryId: Copy>,
    P: AsRef<str> + AsRef<[u8]>,
{
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(
        mpc_contract_id = %self.mpc_contract_id,
        domain_id = self.domain_id,
    )))]
    async fn derive_sign_recoverable(
        &self,
        path: P,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error>
    where
        P: Send,
    {
        let path: &str = path.as_ref();
        self.sign_extract(
            path,
            C::msg_to_payload(msg).ok_or(Error::InvalidPayload)?,
            {
                let derived_public_key = LazyCell::new(|| self.derive_public_key(path));
                move |sig| {
                    let (sig, rec_id) = C::parse_recoverable_signature(sig)?;
                    let recovered_public_key = C::recover(msg, &sig, rec_id)?;
                    if recovered_public_key != *derived_public_key {
                        // the signature is either invalid or not ours
                        return None;
                    }
                    Some((sig, rec_id))
                }
            },
        )
        .await
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("invalid domain: {0}")]
    InvalidDomain(u64),

    #[error("invalid sign payload")]
    InvalidPayload,

    #[error("NEAR: {0}")]
    Near(#[from] near_kit::Error),

    #[error(transparent)]
    Sender(Box<dyn StdError + Send + Sync>),

    #[error("no matching signature was found in transaction '{0}'")]
    SignatureNotFound(CryptoHash),
}
