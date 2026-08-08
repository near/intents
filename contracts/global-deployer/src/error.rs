/// An error for [`GlobalDeployer`](crate::contract::GlobalDeployer) contract.
#[cfg_attr(feature = "near-contract", derive(::near_sdk::FunctionError))]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unauthorized")]
    Unauthorized,
    #[error("invalid code hash")]
    InvalidCodeHash,
    #[error("requires exactly 1 yoctoNEAR attached")]
    RequireOneYocto,
    #[error("self-transfer")]
    SelfTransfer,
}
