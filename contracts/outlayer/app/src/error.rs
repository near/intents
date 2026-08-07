/// An error for [`OutlayerApp`](crate::contract::OutlayerApp) contract.
#[cfg_attr(feature = "near-contract", derive(::near_sdk::FunctionError))]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("self-transfer")]
    SelfTransfer,
    #[error("unauthorized")]
    Unauthorized,
    #[error("old_hash mismatch")]
    WrongCodeHash,
    #[error("insufficient attached deposit")]
    InsufficientDeposit,
    #[error("requires exactly 1 yoctoNEAR attached")]
    RequireOneYocto,
}
