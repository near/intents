use std::{collections::BTreeMap, fmt::Debug};

use futures::{FutureExt, future::BoxFuture};
use near_api::{
    errors::ExecuteTransactionError, types::{
        errors::{DataConversionError, ExecutionError}, transaction::{
            actions::{
                AddKeyAction, CreateAccountAction, DeployContractAction, DeployGlobalContractAction, DeterministicAccountStateInit, DeterministicAccountStateInitV1, DeterministicStateInitAction, FunctionCallAction, GlobalContractDeployMode, GlobalContractIdentifier, TransferAction, UseGlobalContractAction
            },
            result::{ExecutionFinalResult, ValueOrReceiptId},
        }, AccessKey, AccessKeyPermission, Action
    }, PublicKey, Transaction
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize}, serde::{de::DeserializeOwned, Serialize}, serde_json, AccountId, Gas, NearToken
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

    pub fn state_init(self, global_contract: AccountId, state: BTreeMap<Vec<u8>, Vec<u8>>)  -> TxBuilder {
        // let StateInit::V1(StateInitV1 { code, data, .. }) = state;
        //
        // self.add_action(
        //     Action::DeployGlobalContract(DeployGlobalContractAction { code, deploy_mode }),
        //     |_| Ok(()),
        // );
        //
        self.add_action(
            Action::DeterministicStateInit(Box::new(DeterministicStateInitAction 
                { 
                state_init: DeterministicAccountStateInit::V1( DeterministicAccountStateInitV1{ 
                    code: GlobalContractIdentifier::AccountId(global_contract), 
                    data: state, 
                }),
                deposit: NearToken::from_near(0) 
            })),
            |_| Ok(()))
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
            let tx_result = Transaction::construct(self.signer.id().clone(), self.receiver_id)
                .add_actions(self.actions)
                .with_signer(self.signer.signer())
                .send_to(self.signer.network_config())
                .await
                .inspect(|r| eprintln!("{:#?}", TxOutcome(r)))?;

            // Collect logs before consuming tx_result
            let logs: Vec<_> = tx_result
                .outcomes()
                .iter()
                .filter(|o| !o.logs.is_empty())
                .map(|o| (o.executor_id.clone(), o.logs.clone()))
                .collect();

            let result = tx_result
                .into_result()
                .inspect_err(|_| {
                    // Print logs from all outcomes on error
                    for (executor_id, outcome_logs) in &logs {
                        eprintln!("Logs from {executor_id}: {outcome_logs:?}");
                    }
                })
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
    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Header: signer -> receiver
        writeln!(
            f,
            "\n{} -> {}",
            self.0.transaction().signer_id(),
            self.0.transaction().receiver_id()
        )?;

        // Display transaction actions
        let actions = self.0.transaction().actions();
        for action in actions {
            if let Action::FunctionCall(fc) = action {
                writeln!(f, "  function: {}", fc.method_name)?;

                let args_str = String::from_utf8_lossy(&fc.args);
                // Pretty print JSON if possible
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&args_str) {
                    let pretty = serde_json::to_string_pretty(&json).unwrap_or_else(|_| args_str.into_owned());
                    for line in pretty.lines() {
                        writeln!(f, "    {line}")?;
                    }
                } else if !args_str.is_empty() {
                    let args_display = if args_str.len() > 200 {
                        format!("{}...", &args_str[..200])
                    } else {
                        args_str.into_owned()
                    };
                    writeln!(f, "    args: {args_display}")?;
                }
                writeln!(f, "  gas_limit: {}, deposit: {}", fc.gas, fc.deposit)?;
            }
        }

        // Calculate total gas
        let total_gas: u64 = self.0.outcomes().iter().map(|o| o.gas_burnt.as_gas()).sum();
        writeln!(f, "  total_gas: {:.2} TGas", total_gas as f64 / 1e12)?;

        // Outcomes
        writeln!(f, "  outcomes:")?;
        for outcome in self.0.outcomes() {
            write!(f, "    {} ({:.2} TGas)", outcome.executor_id, outcome.gas_burnt.as_gas() as f64 / 1e12)?;

            // Show result status
            match outcome.clone().into_result() {
                Ok(v) => {
                    if let ValueOrReceiptId::Value(value) = v {
                        if let Ok(bytes) = value.raw_bytes() {
                            if !bytes.is_empty() {
                                write!(f, " -> OK")?;
                                // Try to decode as JSON
                                if let Ok(s) = String::from_utf8(bytes.clone()) {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
                                        write!(f, ": {json}")?;
                                    } else if !s.is_empty() {
                                        write!(f, ": {s}")?;
                                    }
                                } else {
                                    write!(f, ": {bytes:?}")?;
                                }
                            }
                        }
                    }
                }
                Err(err) => write!(f, " -> FAIL: {err}")?,
            }
            writeln!(f)?;

            // Show logs indented
            for log in &outcome.logs {
                writeln!(f, "      log: {log}")?;
            }
        }

        Ok(())
    }
}

