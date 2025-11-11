use std::collections::{BTreeMap, BTreeSet};

use crate::{Error, Price, Result, SendParams};

use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_fees::Pips;
use defuse_near_utils::time::Deadline;
use defuse_num_utils::CheckedAdd;
use defuse_token_id::TokenId;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, borsh, env, near};
use serde_with::{DisplayFromStr, hex::Hex, serde_as};

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
        // TODO: remove
        const PREFIX: &str = "escrow-";

        let factory = factory.as_ref();

        let serialized = borsh::to_vec(self).unwrap_or_else(|_| unreachable!());
        let hash = env::keccak256_array(&serialized);

        let len = AccountId::MAX_LEN - 1 - factory.len() - PREFIX.len();
        format!("{PREFIX}{}.{factory}", hex::encode(&hash[32 - len / 2..32]))
            .parse()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn verify(&self, fixed: &FixedParams) -> Result<()> {
        if fixed.hash() != self.fixed_params_hash {
            return Err(Error::WrongData);
        }

        if fixed.src_asset == fixed.dst_asset {
            return Err(Error::SameAsset);
        }

        if fixed.total_fee().ok_or(Error::ExcessiveFees)? >= Pips::MAX {
            return Err(Error::ExcessiveFees);
        }

        Ok(())
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

    // TODO: check != src_asset
    pub src_asset: TokenId,
    pub dst_asset: TokenId,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub refund_src_to: SendParams,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub receive_dst_to: SendParams,

    #[serde(default)]
    pub partial_fills_allowed: bool,

    // TODO: check that fees are non-zero
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fees: BTreeMap<AccountId, Pips>,

    // TODO: store only merkle root? leaves have salts
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub taker_whitelist: BTreeSet<AccountId>,
    // TODO: whitelist: Option<signer_id>

    // TODO: authority
    // TODO: close authority: intents adapter for on_auth

    // allows:
    //   * price update (solver message: min_price)
    //   * deadline update (short)
    //   * cancel before deadline (longer, shorter)
    // TODO: allow .on_auth()
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // TODO: can it then be a deteministic contract supporting multisig or any-of functionality
    // pub maker_authority: Option<AccountId>,
    // TODO: salt?
    // TODO: refund_to
}

impl FixedParams {
    pub fn total_fee(&self) -> Option<Pips> {
        self.fees
            .iter()
            .map(|(_, fee)| *fee)
            .try_fold(Pips::ZERO, |total, fee| total.checked_add(fee))
    }
}

impl FixedParams {
    pub fn hash(&self) -> CryptoHash {
        // TODO: prefix?
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
    /// TODO: exact out? i.e. partial fills are not allowed
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
    // #[serde_as(as = "SerdeTimestampNanoSeconds")] // TODO: RFC-3339
    pub deadline: Deadline,
}

// TODO: (Optional but nice) bump a version so indexers/UIs know the latest state

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
    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub closed: bool,

    /// Deposited or lost (after close) src remaining
    #[serde_as(as = "DisplayFromStr")]
    pub maker_src_remaining: u128,

    // Store only lost for maker, since we're bounded in state size
    // So, we don't store lost&found for takers and fee_collectors
    #[serde_as(as = "DisplayFromStr")]
    pub maker_dst_lost: u128,

    #[serde(skip)]
    pub callbacks_in_flight: u32,
    // TODO: lost_found: store zero for beging transfer, otherwise - fail
}

// TODO: CoW schema:
// {
//    "uid":"0xaa4eb7b4da14b93ce42963ac4085fd8eee4a04170b36454f9f8b91b91f69705387a04752e5//16548b0d5d4df97384c0b22b64917965a801c1",
//    "sellToken": "0xdef1ca1fb7fbcdc777520aa7f396b4e015f497ab",
//    "buyToken": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
//    "sellAmount": "1000000000000000000000",
//    "buyAmount": "284138335",
//    "feeAmount": "0",
//    "kind": "sell",
//    "partiallyFillable": false,
//    "class": "limit"
//}
