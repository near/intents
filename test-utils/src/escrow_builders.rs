//! Builders for escrow swap tests.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use defuse_escrow_swap::{
    Deadline, OverrideSend, Params, Pips, ProtocolFees,
    action::{FillAction, TransferAction, TransferMessage},
    decimal::UD128,
};

const DEFAULT_DEADLINE_SECS: u64 = 360;
/// Default salt for test/example purposes. Not suitable for production.
const ZERO_SALT: [u8; 32] = [0u8; 32];
use defuse_token_id::TokenId;
use near_sdk::AccountId;

/// Builder for creating escrow swap parameters.
/// Takes (maker, `src_token`) and (takers, `dst_token`) tuples associating actors with their tokens.
#[derive(Debug, Clone)]
pub struct ParamsBuilder {
    maker: AccountId,
    src_token: TokenId,
    takers: BTreeSet<AccountId>,
    dst_token: TokenId,
    salt: Option<[u8; 32]>,
    price: Option<UD128>,
    partial_fills_allowed: Option<bool>,
    deadline: Option<Deadline>,
    refund_src_to: Option<OverrideSend>,
    receive_dst_to: Option<OverrideSend>,
    auth_caller: Option<AccountId>,
    protocol_fees: Option<ProtocolFees>,
    integrator_fees: BTreeMap<AccountId, Pips>,
}

impl ParamsBuilder {
    pub fn new(
        (maker, src_token): (AccountId, TokenId),
        (takers, dst_token): (impl IntoIterator<Item = AccountId>, TokenId),
    ) -> Self {
        Self {
            maker,
            src_token,
            takers: takers.into_iter().collect(),
            dst_token,
            salt: None,
            price: None,
            partial_fills_allowed: None,
            deadline: None,
            refund_src_to: None,
            receive_dst_to: None,
            auth_caller: None,
            protocol_fees: None,
            integrator_fees: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn with_salt(mut self, salt: [u8; 32]) -> Self {
        self.salt = Some(salt);
        self
    }

    #[must_use]
    pub const fn with_price(mut self, price: UD128) -> Self {
        self.price = Some(price);
        self
    }

    #[must_use]
    pub const fn with_partial_fills_allowed(mut self, allowed: bool) -> Self {
        self.partial_fills_allowed = Some(allowed);
        self
    }

    #[must_use]
    pub const fn with_deadline(mut self, deadline: Deadline) -> Self {
        self.deadline = Some(deadline);
        self
    }

    #[must_use]
    pub fn with_refund_src_to(mut self, refund_src_to: OverrideSend) -> Self {
        self.refund_src_to = Some(refund_src_to);
        self
    }

    #[must_use]
    pub fn with_receive_dst_to(mut self, receive_dst_to: OverrideSend) -> Self {
        self.receive_dst_to = Some(receive_dst_to);
        self
    }

    #[must_use]
    pub fn with_auth_caller(mut self, auth_caller: AccountId) -> Self {
        self.auth_caller = Some(auth_caller);
        self
    }

    #[must_use]
    pub fn with_protocol_fees(mut self, protocol_fees: ProtocolFees) -> Self {
        self.protocol_fees = Some(protocol_fees);
        self
    }

    #[must_use]
    pub fn with_integrator_fee(mut self, account_id: AccountId, fee: Pips) -> Self {
        self.integrator_fees.insert(account_id, fee);
        self
    }

    /// Returns the takers whitelist.
    pub const fn takers(&self) -> &BTreeSet<AccountId> {
        &self.takers
    }

    pub fn build(self) -> Params {
        Params {
            maker: self.maker,
            src_token: self.src_token,
            dst_token: self.dst_token,
            price: self.price.unwrap_or(UD128::ONE),
            deadline: self.deadline.unwrap_or_else(|| {
                Deadline::timeout(std::time::Duration::from_secs(DEFAULT_DEADLINE_SECS))
            }),
            partial_fills_allowed: self.partial_fills_allowed.unwrap_or(false),
            refund_src_to: self.refund_src_to.unwrap_or_default(),
            receive_dst_to: self.receive_dst_to.unwrap_or_default(),
            taker_whitelist: self.takers,
            protocol_fees: self.protocol_fees,
            integrator_fees: self.integrator_fees,
            auth_caller: self.auth_caller,
            salt: self.salt.unwrap_or(ZERO_SALT),
        }
    }
}

pub struct FundMessageBuilder {
    params: Params,
}

impl FundMessageBuilder {
    #[must_use]
    pub const fn new(params: Params) -> Self {
        Self { params }
    }

    #[must_use]
    pub fn build(self) -> TransferMessage {
        TransferMessage {
            params: self.params,
            action: TransferAction::Fund,
        }
    }
}

/// Builder for creating Fill transfer messages.
pub struct FillMessageBuilder {
    params: Params,
    price: Option<UD128>,
    deadline: Option<Deadline>,
    receive_src_to: Option<OverrideSend>,
}

impl FillMessageBuilder {
    #[must_use]
    pub const fn new(params: Params) -> Self {
        Self {
            params,
            price: None,
            deadline: None,
            receive_src_to: None,
        }
    }

    #[must_use]
    pub const fn with_price(mut self, price: UD128) -> Self {
        self.price = Some(price);
        self
    }

    #[must_use]
    pub const fn with_deadline(mut self, deadline: Deadline) -> Self {
        self.deadline = Some(deadline);
        self
    }

    #[must_use]
    pub fn with_receive_src_to(mut self, receive_src_to: OverrideSend) -> Self {
        self.receive_src_to = Some(receive_src_to);
        self
    }

    #[must_use]
    pub fn build(self) -> TransferMessage {
        TransferMessage {
            params: self.params.clone(),
            action: TransferAction::Fill(FillAction {
                price: self.price.unwrap_or(self.params.price),
                deadline: self.deadline.unwrap_or_else(|| {
                    Deadline::timeout(std::time::Duration::from_secs(DEFAULT_DEADLINE_SECS))
                }),
                receive_src_to: self.receive_src_to.unwrap_or_default(),
            }),
        }
    }
}
