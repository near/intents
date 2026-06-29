mod message;
mod nonces;
mod request;
mod state;

pub use self::{message::*, nonces::*, request::*, state::*};

pub use defuse_time::Timestamp;
pub use near_account_id::{AccountId, AccountIdRef};

pub const WALLET_DOMAIN: &[u8] = b"NEAR_WALLET_CONTRACT/V1";
