use std::fmt::Debug;

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
use near_sdk::{
    AccountId, Gas, NearToken,
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Serialize, de::DeserializeOwned},
    serde_json,
};
use thiserror::Error as ThisError;

use crate::SigningAccount;

pub struct TxBuilder<T = ()> {
    signer: SigningAccount,
    receiver_id: AccountId,

    actions: Vec<Action>,

    result: fn(Vec<u8>) -> Result<T, DataConversionError>,
}

impl TxBuilder<()> {
    pub const fn new(signer: SigningAccount, receiver_id: AccountId) -> Self {
        Self {
            signer,
            receiver_id,
            actions: Vec::new(),
            result: |_| Ok(()),
        }
    }
}

impl<T> TxBuilder<T> {
    pub fn create_account(self) -> TxBuilder {
        self.add_action(Action::CreateAccount(CreateAccountAction {}), |_| Ok(()))
    }

    pub fn transfer(self, deposit: NearToken) -> TxBuilder {
        self.add_action(Action::Transfer(TransferAction { deposit }), |_| Ok(()))
    }

    pub fn deploy(self, code: Vec<u8>) -> TxBuilder {
        self.add_action(
            Action::DeployContract(DeployContractAction { code }),
            |_| Ok(()),
        )
    }

    pub fn deploy_global(self, code: Vec<u8>, deploy_mode: GlobalContractDeployMode) -> TxBuilder {
        self.add_action(
            Action::DeployGlobalContract(DeployGlobalContractAction { code, deploy_mode }),
            |_| Ok(()),
        )
    }

    pub fn use_global(self, global_id: GlobalContractIdentifier) -> TxBuilder {
        self.add_action(
            Action::UseGlobalContract(
                UseGlobalContractAction {
                    contract_identifier: global_id,
                }
                .into(),
            ),
            |_| Ok(()),
        )
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

    fn add_key(self, pk: impl Into<PublicKey>, access_key: AccessKey) -> TxBuilder {
        self.add_action(
            Action::AddKey(
                AddKeyAction {
                    public_key: pk.into(),
                    access_key,
                }
                .into(),
            ),
            |_| Ok(()),
        )
    }

    pub fn function_call_json<R>(
        self,
        name: impl Into<String>,
        args: impl Serialize,
        gas: Gas,
        deposit: NearToken,
    ) -> TxBuilder<R>
    where
        R: DeserializeOwned,
    {
        self.function_call(
            name,
            serde_json::to_vec(&args).unwrap(),
            gas,
            deposit,
            |bytes| serde_json::from_slice(&bytes).map_err(Into::into),
        )
    }

    pub fn function_call_borsh<R>(
        self,
        name: impl Into<String>,
        args: impl BorshSerialize,
        gas: Gas,
        deposit: NearToken,
    ) -> TxBuilder<R>
    where
        R: BorshDeserialize,
    {
        self.function_call(name, borsh::to_vec(&args).unwrap(), gas, deposit, |bytes| {
            borsh::from_slice(&bytes).map_err(Into::into)
        })
    }

    fn function_call<R>(
        self,
        name: impl Into<String>,
        args: Vec<u8>,
        gas: Gas,
        deposit: NearToken,
        result: fn(Vec<u8>) -> Result<R, DataConversionError>,
    ) -> TxBuilder<R> {
        self.add_action(
            Action::FunctionCall(
                FunctionCallAction {
                    method_name: name.into(),
                    args,
                    gas,
                    deposit,
                }
                .into(),
            ),
            result,
        )
    }

    fn add_action<R>(
        mut self,
        action: Action,
        result: fn(Vec<u8>) -> Result<R, DataConversionError>,
    ) -> TxBuilder<R> {
        self.actions.push(action);
        self.map(result)
    }

    fn map<R>(self, result: fn(Vec<u8>) -> Result<R, DataConversionError>) -> TxBuilder<R> {
        TxBuilder {
            signer: self.signer,
            receiver_id: self.receiver_id,
            actions: self.actions,
            result,
        }
    }

    pub fn no_result(self) -> TxBuilder {
        self.map(|_| Ok(()))
    }
}

impl<T> IntoFuture for TxBuilder<T>
where
    T: 'static,
{
    type Output = Result<T, TxError>;

    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            let result = Transaction::construct(self.signer.id().clone(), self.receiver_id)
                .add_actions(self.actions)
                .with_signer(self.signer.signer())
                .send_to(self.signer.network_config())
                .await
                .inspect(|r| eprintln!("{:#?}", TxOutcome(r)))?
                .into_result()
                .map_err(Box::new)
                .map_err(ExecutionError::TransactionFailure)?
                .raw_bytes()?;

            (self.result)(result)
                .map_err(Into::<ExecutionError>::into)
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

struct TxOutcome<'a>(&'a ExecutionFinalResult);

impl Debug for TxOutcome<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} -> {}: ",
            self.0.transaction().signer_id(),
            self.0.transaction().receiver_id()
        )?;
        let outcomes: Vec<_> = self
            .0
            .outcomes()
            .into_iter()
            .map(TestExecutionOutcome)
            .collect();
        if !outcomes.is_empty() {
            f.debug_list().entries(outcomes).finish()?;
        }
        Ok(())
    }
}

struct TestExecutionOutcome<'a>(&'a ExecutionOutcome);

impl Debug for TestExecutionOutcome<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ({}) ", self.0.executor_id, self.0.gas_burnt)?;
        if !self.0.logs.is_empty() {
            f.debug_list().entries(&self.0.logs).finish()?;
        }
        match self.0.clone().into_result() {
            Ok(v) => {
                if let ValueOrReceiptId::Value(value) = v {
                    let bytes = value.raw_bytes().unwrap();
                    if !bytes.is_empty() {
                        write!(f, ", OK: {:?}", bytes)?;
                    }
                }
                Ok(())
            }
            Err(err) => write!(f, ", FAIL: {err:#?}"),
        }
    }
}
