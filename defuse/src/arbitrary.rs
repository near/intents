use near_sdk::{Gas, NearToken, ext_contract, near};

#[near(serializers = [json])]
pub enum ArbitraryAction {
    FunctionCall {
        function_name: String,
        arguments: Vec<u8>,
        amount: NearToken,
        gas: Gas,
    },
    Transfer {
        amount: NearToken,
    },
}

#[ext_contract(ext_arbitrary_manager)]
#[allow(clippy::module_name_repetitions)]
pub trait ArbitraryManager {
    /// Allows the caller to execute an arbitrary function call
    /// or transfer on the contract.
    fn arbitrary_call(&mut self, action: ArbitraryAction);
}
