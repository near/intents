mod convert;
#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;
mod sender;
mod types;

pub use self::{convert::*, sender::*};

use std::{cell::LazyCell, fmt::Debug};

use defuse_kdf::{
    Additive, Derive, DeriveExt, DeriveSigner, RecoverableDeriveSigner,
    crypto::{Curve, RecoverableCurve},
};
use defuse_mpc_kdf::TweakSchema;
use defuse_near_promise::{AccountId, AccountIdRef, Gas, NearToken, actions::FunctionCall};
use impl_tools::autoimpl;
use near_kit::{ExecutedOptimistic, ExecutionStatus, RpcClient, WaitLevel};

use crate::types::{SignRequest, SignResponse};

// TODO: docs
pub const MAINNET_MPC_CONTRACT_ID: &AccountIdRef = AccountIdRef::new_or_panic("v1.signer");

#[autoimpl(Debug, Clone where C::PublicKey: trait, S: trait)]
pub struct MpcOnChainSigner<C: Curve, S> {
    sender: S,

    mpc_contract_id: AccountId,
    mpc_public_key: C::PublicKey,
    domain_id: u64,

    client: RpcClient,
}

impl<C, S> MpcOnChainSigner<C, S>
where
    C: Curve,
    S: Sender,
{
    pub async fn new(
        sender: S,
        mpc_contract_id: impl Into<AccountId>,
        domain_id: u64,
        client: RpcClient,
    ) -> Self {
        todo!()
    }
}

impl<C, P, S> DeriveSigner<C, P> for MpcOnChainSigner<C, S>
where
    C: OnChainNearMpcCurve<PublicKey: Clone + Send + Sync>,
    P: AsRef<str> + AsRef<[u8]>, // TODO
    S: Sender,
{
    type Error = Error<S::Error>;

    type Schema<'a>
        = Derive<Additive<C>, TweakSchema<C>>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        Additive::new(self.mpc_public_key.clone())
            .derive(defuse_mpc_kdf::tweak(self.sender.account_id()))
    }

    // TODO: tracing
    async fn derive_sign(&self, path: P, msg: &[u8]) -> Result<C::Signature, Self::Error>
    where
        P: Send,
    {
        let path: &str = path.as_ref();

        let call = FunctionCall::name("sign")
            .attach_deposit(NearToken::from_yoctonear(1))
            .args_json(SignRequest {
                path: path.into(),
                payload: C::to_payload(msg).ok_or(Error::InvalidPayload)?,
                domain_id: self.domain_id,
            })
            // TODO: is it enough?
            .gas(Gas::from_tgas(10));

        let sent_tx = self
            .sender
            .send(self.mpc_contract_id.clone(), vec![call.into()])
            .await
            .map_err(Error::Sender)?;

        let tx_status = self
            .client
            .tx_status(
                &sent_tx.tx_hash.into(),
                &sent_tx.sender_id,
                // wait for `mpc_contract_id::sign()` receipt to execute
                near_kit::TxExecutionStatus::ExecutedOptimistic,
            )
            .await
            .map_err(near_kit::Error::from)?;

        let tx_outcome = ExecutedOptimistic::convert(tx_status, &sent_tx.sender_id)?;

        let derived_public_key = LazyCell::new(|| self.derive_public_key(path));
        for outcome in tx_outcome.receipts_outcome {
            if outcome.outcome.executor_id != self.mpc_contract_id {
                continue;
            }
            let ExecutionStatus::SuccessValue(value) = outcome.outcome.status else {
                continue;
            };
            let Ok(sig) = serde_json::from_slice::<SignResponse>(&value) else {
                continue;
            };
            let Some(sig) = C::parse_signature(sig) else {
                continue;
            };
            if C::verify(&derived_public_key, msg, &sig) {
                // TODO: tracing: receipt_id
                return Ok(sig);
            }
        }

        return Err(Error::SignatureNotFound);
    }
}

impl<C, P, S> RecoverableDeriveSigner<C, P> for MpcOnChainSigner<C, S>
where
    C: RecoverableCurve,
    P: AsRef<str> + AsRef<[u8]>,
    S: Sender,
{
    async fn derive_sign_recoverable(
        &self,
        path: P,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error>
    where
        P: Send,
    {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error<S> {
    #[error("invalid sign payload")]
    InvalidPayload,

    #[error("NEAR: {0}")]
    Near(#[from] near_kit::Error),

    #[error(transparent)]
    Sender(S),

    // TODO: provide tx hash
    #[error("signature was not found in transaction")]
    SignatureNotFound,
}

// TODO
// impl<C, P, S> RecoverableDeriveSigner<C, P> for MpcOnChainSigner<S>
// where
//     C: NearMpcCurve + RecoverableCurve,
//     P: AsRef<str> + AsRef<[u8]>, // TODO
//     S: Sender,
// {
//     async fn derive_sign_recoverable(
//         &self,
//         path: P,
//         msg: &[u8],
//     ) -> Result<(C::Signature, C::RecoveryId), Self::Error>
//     where
//         P: Send,
//     {
//         todo!()
//     }
// }

// use near_kit::{Error, Included, Near, SendTxResponse};

// impl Relayer for Near {
//     type Error = Error;

//     async fn send(
//         &self,
//         receiver_id: AccountId,
//         actions: Vec<NearAction>,
//     ) -> Result<SentTransaction, Self::Error> {
//         // TODO: reuse logic from relayer

//         actions
//             .into_iter()
//             .fold(self.transaction(receiver_id), |tx, action| {
//                 tx.add_action(action)
//             })
//             .send()
//             // TODO: maybe IncludedFinal?
//             .wait_until(Included)
//             // TODO: max_nonce_retries
//             .await
//             .map(Into::into)
//     }
// }

// impl Sender for Near {
//     #[inline]
//     fn account_id(&self) -> Cow<'_, AccountIdRef> {
//         self.account_id().into()
//     }
// }
