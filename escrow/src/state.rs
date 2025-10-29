use std::collections::{BTreeMap, BTreeSet};

use crate::{Error, Price, Result};

use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_fees::Pips;
use defuse_token_id::nep245::Nep245TokenId as TokenId;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, borsh, env, near};
use serde_with::{
    DisplayFromStr, TimestampNanoSeconds as SerdeTimestampNanoSeconds, hex::Hex, serde_as,
};

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]

pub struct Storage {
    #[serde_as(as = "Hex")]
    pub fixed_params_hash: [u8; 32],

    #[serde(flatten)]
    pub params: Params,

    #[serde(flatten)]
    pub state: State,
}

impl Storage {
    pub fn new(fixed: &FixedParams, params: Params) -> Self {
        Self {
            fixed_params_hash: fixed.hash(),
            params,
            state: State::default(),
        }
    }

    // TODO: nep616 feature
    pub fn derive_account_id(&self, factory: impl AsRef<AccountIdRef>) -> AccountId {
        let factory = factory.as_ref();

        let serialized = borsh::to_vec(self).unwrap_or_else(|_| unreachable!());
        let hash = env::keccak256_array(&serialized);

        let len = AccountId::MAX_LEN - 1 - factory.len();
        format!("{}.{factory}", hex::encode(&hash[32 - len / 2..32]))
            .parse()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn verify(&self, fixed: &FixedParams) -> Result<()> {
        (fixed.hash() == self.fixed_params_hash)
            .then_some(())
            .ok_or(Error::WrongData)
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct FixedParams {
    pub maker: AccountId,

    // TODO: nep245: token_id length is less than max on intents.near
    // TODO: check != src_asset
    #[serde_as(as = "DisplayFromStr")]
    pub src_asset: TokenId,
    #[serde_as(as = "DisplayFromStr")]
    pub dst_asset: TokenId,

    // TODO: maker msg
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maker_dst_receiver_id: Option<AccountId>,

    #[serde(default)]
    pub partial_fills_allowed: bool,

    // TODO: check that fees are non-zero
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fees: BTreeMap<AccountId, Pips>,

    // TODO: store only merkle root? leaves have salts
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub taker_whitelist: BTreeSet<AccountId>,
    // TODO: whitelist: Option<signer_id>

    // allows:
    //   * price update (solver message: min_price)
    //   * deadline update (short)
    //   * cancel before deadline (longer, shorter)
    // TODO: allow .on_auth()
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maker_authority: Option<AccountId>,
    // TODO: salt?
    // TODO: refund_to
}

impl FixedParams {
    pub fn hash(&self) -> CryptoHash {
        env::keccak256_array(&borsh::to_vec(self).unwrap_or_else(|_| unreachable!()))
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct Params {
    /// maker / taker (in 10^-9)
    /// TODO: check non-zero
    pub price: Price,

    // TODO: check that not expired at create?
    #[borsh(
        serialize_with = "BorshAs::<BorshTimestampNanoSeconds>::serialize",
        deserialize_with = "BorshAs::<BorshTimestampNanoSeconds>::deserialize",
        schema(with_funcs(
            declaration = "i64::declaration",
            definitions = "i64::add_definitions_recursively",
        ))
    )]
    #[serde_as(as = "SerdeTimestampNanoSeconds")] // TODO: RFC-3339
    pub deadline: DateTime<Utc>,
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Default, Clone)]
pub struct State {
    #[serde_as(as = "DisplayFromStr")]
    pub src_remaining: u128,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub closed: bool,
    // TODO: lost_found: store zero for beging transfer, otherwise - fail
}
