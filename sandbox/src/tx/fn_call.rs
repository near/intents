use near_api::types::transaction::actions::FunctionCallAction;
use near_sdk::{
    Gas, NearToken,
    borsh::{self, BorshSerialize},
    serde, serde_json,
};

const DEFAULT_GAS: Gas = Gas::from_tgas(300);
const NO_DEPOSIT: NearToken = NearToken::from_yoctonear(0);

pub struct FnCallBuilder {
    name: &'static str,
    args: Vec<u8>,
    gas: Gas,
    deposit: NearToken,
}

impl FnCallBuilder {
    pub fn new(name: &'static str) -> Self {
        Self {
            name: name,
            args: Vec::new(),
            //  serde_json::to_string(&serde_json::json!({}))
            //     .unwrap()
            //     .into_bytes(),
            gas: DEFAULT_GAS,
            deposit: NO_DEPOSIT,
        }
    }

    #[must_use]
    pub const fn with_gas(mut self, gas: Gas) -> Self {
        self.gas = gas;
        self
    }

    #[must_use]
    pub const fn with_deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }

    #[must_use]
    pub fn raw_args(mut self, args: impl Into<Vec<u8>>) -> Self {
        self.args = args.into();
        self
    }

    #[must_use]
    pub fn json_args<T: serde::Serialize>(mut self, args: T) -> Self {
        self.args = serde_json::to_vec(&args).unwrap();
        self
    }

    #[must_use]
    pub fn borsh_args<T: BorshSerialize>(mut self, args: &T) -> Self {
        self.args = borsh::to_vec(args).unwrap();
        self
    }
}

impl From<FnCallBuilder> for FunctionCallAction {
    fn from(value: FnCallBuilder) -> Self {
        Self {
            method_name: value.name.to_string(),
            args: value.args,
            gas: value.gas,
            deposit: value.deposit,
        }
    }
}
