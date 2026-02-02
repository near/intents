use std::collections::BTreeMap;

use futures::{FutureExt, future::BoxFuture};
use near_api::{
    PublicKey, Transaction,
    types::{
        AccessKey, AccessKeyPermission, Action,
        transaction::{
            actions::{
                AddKeyAction, CreateAccountAction, DeployContractAction,
                DeployGlobalContractAction, DeterministicAccountStateInit,
                DeterministicAccountStateInitV1, DeterministicStateInitAction, FunctionCallAction,
                GlobalContractDeployMode, GlobalContractIdentifier, TransferAction,
                UseGlobalContractAction,
            },
            result::{ExecutionFinalResult, ExecutionSuccess},
        },
    },
};
use near_sdk::{AccountId, NearToken};

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
    pub(crate) fn new(signer: SigningAccount, receiver_id: impl Into<AccountId>) -> Self {
        Self {
            signer,
            receiver_id: receiver_id.into(),
            actions: Vec::new(),
        }
    }

    #[must_use]
    pub fn create_account(self) -> Self {
        self.add_action(Action::CreateAccount(CreateAccountAction {}))
    }

    #[must_use]
    pub fn transfer(self, deposit: NearToken) -> Self {
        self.add_action(Action::Transfer(TransferAction { deposit }))
    }

    #[must_use]
    pub fn deploy(self, code: impl Into<Vec<u8>>) -> Self {
        self.add_action(Action::DeployContract(DeployContractAction {
            code: code.into(),
        }))
    }

    #[must_use]
    pub fn deploy_global(
        self,
        code: impl Into<Vec<u8>>,
        deploy_mode: GlobalContractDeployMode,
    ) -> Self {
        self.add_action(Action::DeployGlobalContract(DeployGlobalContractAction {
            code: code.into(),
            deploy_mode,
        }))
    }

    #[must_use]
    pub fn use_global(self, global_id: GlobalContractIdentifier) -> Self {
        self.add_action(Action::UseGlobalContract(
            UseGlobalContractAction {
                contract_identifier: global_id,
            }
            .into(),
        ))
    }

    #[must_use]
    pub fn state_init(self, global_contract: AccountId, state: BTreeMap<Vec<u8>, Vec<u8>>) -> Self {
        self.add_action(Action::DeterministicStateInit(Box::new(
            DeterministicStateInitAction {
                state_init: DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
                    code: GlobalContractIdentifier::AccountId(global_contract),
                    data: state,
                }),
                deposit: NearToken::from_near(0),
            },
        )))
    }

    #[must_use]
    pub fn add_full_access_key(self, pk: impl Into<PublicKey>) -> Self {
        self.add_key(
            pk,
            AccessKey {
                nonce: 0.into(),
                permission: AccessKeyPermission::FullAccess,
            },
        )
    }

    #[must_use]
    pub fn function_call(self, action: impl Into<FunctionCallAction>) -> Self {
        self.add_action(Action::FunctionCall(action.into().into()))
    }

    #[must_use]
    fn add_key(self, pk: impl Into<PublicKey>, access_key: AccessKey) -> Self {
        self.add_action(Action::AddKey(
            AddKeyAction {
                public_key: pk.into(),
                access_key,
            }
            .into(),
        ))
    }

    #[must_use]
    fn add_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    // Use this only if there is a need to get execution result - otherwise use awaiting TxBuilder directly
    pub async fn exec_transaction(self) -> anyhow::Result<ExecutionFinalResult> {
        Transaction::construct(self.signer.id().clone(), self.receiver_id)
            .add_actions(self.actions)
            .with_signer(self.signer.signer().clone())
            .send_to(self.signer.network_config())
            .await
            .inspect(|r| eprintln!("{:#?}", TxOutcome::from(r)))
            .map_err(Into::into)
    }
}

impl IntoFuture for TxBuilder {
    type Output = anyhow::Result<ExecutionSuccess>;

    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            self.exec_transaction()
                .await?
                .into_result()
                .map_err(Into::into)
        }
        .boxed()
    }
}
