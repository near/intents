use defuse_wallet::signature::RequestMessage;
use serde::Serialize;

#[near_kit::contract]
pub trait Wallet {
    #[call]
    fn w_execute_signed(&mut self, args: WExecuteSignedArgs<'_>);
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WExecuteSignedArgs<'a> {
    pub msg: &'a RequestMessage,
    pub proof: &'a str,
}
