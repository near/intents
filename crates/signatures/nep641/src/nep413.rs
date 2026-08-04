use std::iter;

use defuse_nep413::Nep413Payload;
use itertools::Itertools;

use crate::OffchainMessage;

impl From<OffchainMessage> for Nep413Payload {
    /// Convert into NEP-413 payload
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_nep641::{OffchainMessage, Timestamp};
    /// use defuse_nep413::Nep413Payload;
    ///
    /// let msg = OffchainMessage {
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "extension.near".parse().unwrap(),
    ///     path: vec![
    ///         "wallet.near".parse().unwrap(),
    ///         "v1.signer".parse().unwrap(),
    ///     ],
    ///     timestamp: Timestamp::now(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    ///
    /// assert_eq!(
    ///     msg.into(),
    ///     Nep413Payload {
    ///         message: "Hello, Near!".to_string(),
    ///     },
    /// );
    /// ```
    fn from(msg: OffchainMessage) -> Self {
        Self {
            recipient: iter::once(&msg.signer_id).chain(&msg.path).join(" -> "),
            nonce: msg.hash(),
            // TODO: here, borsh would be not good
            message: msg.payload,
            callback_url: None,
        }
    }
}
