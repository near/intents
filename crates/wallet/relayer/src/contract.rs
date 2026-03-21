use defuse_wallet::signature::RequestMessage;
use near_sdk::near;

#[near_kit::contract]
pub trait WalletContract {
    #[call]
    fn w_execute_signed(&mut self, args: WExecuteSignedArgs);
}

#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WExecuteSignedArgs {
    pub msg: RequestMessage,
    pub proof: String,
}
