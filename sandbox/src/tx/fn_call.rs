use near_api::types::transaction::actions::FunctionCallAction;
use near_sdk::{
    Gas, NearToken,
    borsh::{self, BorshSerialize},
    serde, serde_json,
};

const DEFAULT_GAS: Gas = Gas::from_tgas(300);
const NO_DEPOSIT: NearToken = NearToken::from_yoctonear(0);

pub struct FnCallBuilder {
    name: String,
    args: Vec<u8>,
    gas: Gas,
    deposit: NearToken,
}

impl FnCallBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: serde_json::to_string(&serde_json::json!({}))
                .unwrap()
                .into_bytes(),
            gas: DEFAULT_GAS,
            deposit: NO_DEPOSIT,
        }
    }

    pub fn with_gas(mut self, gas: Gas) -> Self {
        self.gas = gas;
        self
    }

    pub fn with_deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }

    pub fn raw_args(mut self, args: impl AsRef<[u8]>) -> Self {
        self.args = args.as_ref().to_vec();
        self
    }

    pub fn json_args<T: serde::Serialize>(mut self, args: T) -> Self {
        self.args = serde_json::to_vec(&args).unwrap();
        self
    }

    pub fn borsh_args<T: BorshSerialize>(mut self, args: &T) -> Self {
        self.args = borsh::to_vec(args).unwrap();
        self
    }

    pub fn into_action(self) -> FunctionCallAction {
        self.into()
    }
}

impl From<FnCallBuilder> for FunctionCallAction {
    fn from(value: FnCallBuilder) -> Self {
        FunctionCallAction {
            method_name: value.name,
            args: value.args,
            gas: value.gas,
            deposit: value.deposit,
        }
    }
}
