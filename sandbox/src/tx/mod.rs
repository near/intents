use std::{fmt::Debug, marker::PhantomData};

use futures::{FutureExt, future::BoxFuture};
use near_api::{
    PublicKey, Transaction,
    errors::ExecuteTransactionError,
    types::{
        AccessKey, AccessKeyPermission, Action,
        errors::{DataConversionError, ExecutionError},
        transaction::{
            actions::{
                AddKeyAction, CreateAccountAction, DeployContractAction,
                DeployGlobalContractAction, FunctionCallAction, GlobalContractDeployMode,
                GlobalContractIdentifier, TransferAction, UseGlobalContractAction,
            },
            result::{ExecutionFinalResult, ExecutionOutcome, ValueOrReceiptId},
        },
    },
};
use near_sdk::{AccountId, NearToken};
use thiserror::Error as ThisError;

mod fn_call;
mod wrappers;

pub use fn_call::FnCallBuilder;

use crate::SigningAccount;
use wrappers::TxOutcome;

pub struct TxBuilder {
    signer: SigningAccount,

    receiver_id: AccountId,

    actions: Vec<Action>,
}

impl TxBuilder {
    pub fn new(signer: SigningAccount, receiver_id: AccountId) -> Self {
        Self {
            signer,
            receiver_id,
            actions: Vec::new(),
        }
    }
}

impl TxBuilder {
    pub fn create_account(self) -> TxBuilder {
        self.add_action(Action::CreateAccount(CreateAccountAction {}))
    }

    pub fn transfer(self, deposit: NearToken) -> TxBuilder {
        self.add_action(Action::Transfer(TransferAction { deposit }))
    }

    pub fn deploy(self, code: Vec<u8>) -> TxBuilder {
        self.add_action(Action::DeployContract(DeployContractAction { code }))
    }

    pub fn deploy_global(self, code: Vec<u8>, deploy_mode: GlobalContractDeployMode) -> TxBuilder {
        self.add_action(Action::DeployGlobalContract(DeployGlobalContractAction {
            code,
            deploy_mode,
        }))
    }

    pub fn use_global(self, global_id: GlobalContractIdentifier) -> TxBuilder {
        self.add_action(Action::UseGlobalContract(
            UseGlobalContractAction {
                contract_identifier: global_id,
            }
            .into(),
        ))
    }

    pub fn add_full_access_key(self, pk: impl Into<PublicKey>) -> TxBuilder {
        self.add_key(
            pk,
            AccessKey {
                nonce: 0.into(),
                permission: AccessKeyPermission::FullAccess,
            },
        )
    }

    pub fn function_call(self, action: impl Into<FunctionCallAction>) -> TxBuilder {
        self.add_action(Action::FunctionCall(action.into().into()))
    }

    fn add_key(self, pk: impl Into<PublicKey>, access_key: AccessKey) -> TxBuilder {
        self.add_action(Action::AddKey(
            AddKeyAction {
                public_key: pk.into(),
                access_key,
            }
            .into(),
        ))
    }

    fn add_action(mut self, action: Action) -> TxBuilder {
        self.actions.push(action);
        self
    }
}

impl IntoFuture for TxBuilder {
    type Output = Result<ExecutionFinalResult, TxError>;

    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            Transaction::construct(self.signer.id().clone(), self.receiver_id)
                .add_actions(self.actions)
                .with_signer(self.signer.signer())
                .send_to(self.signer.network_config())
                .await
                .inspect(|r| eprintln!("{:#?}", TxOutcome::from(r)))
                .map_err(Into::into)
        }
        .boxed()
    }
}

pub type TxResult<T, E = TxError> = Result<T, E>;

#[derive(Debug, ThisError)]
pub enum TxError {
    #[error(transparent)]
    ExecuteTransactionError(#[from] ExecuteTransactionError),

    #[error(transparent)]
    ExecutionError(#[from] ExecutionError),
}
