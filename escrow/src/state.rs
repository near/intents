use defuse_fees::Pips;
use defuse_near_utils::time::Deadline;
use defuse_token_id::TokenId;

use std::collections::{BTreeMap, BTreeSet};

use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_num_utils::CheckedAdd;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, Gas, borsh, env, near};
use serde_with::{DisplayFromStr, hex::Hex, serde_as};

use crate::{Error, Price, Result};

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]

pub struct ContractStorage {
    #[serde_as(as = "Hex")]
    fixed_params_hash: [u8; 32],

    #[serde(flatten)]
    storage: Storage,
}

impl ContractStorage {
    pub fn new(fixed: &FixedParams, params: Params) -> Self {
        Self {
            fixed_params_hash: fixed.hash(),
            storage: Storage::new(params),
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

    pub const fn no_verify(&self) -> &Storage {
        &self.storage
    }

    pub const fn no_verify_mut(&mut self) -> &mut Storage {
        &mut self.storage
    }

    pub fn verify(&self, fixed: &FixedParams) -> Result<&Storage> {
        (self.fixed_params_hash == fixed.hash())
            .then_some(&self.storage)
            .ok_or(Error::InvalidData)
    }

    pub fn verify_mut(&mut self, fixed: &FixedParams) -> Result<&mut Storage> {
        self.verify(fixed)?;
        Ok(&mut self.storage)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    #[serde(flatten)]
    pub params: Params,

    #[serde(flatten)]
    pub state: State,
}

impl Storage {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            state: State::default(),
        }
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedParams {
    pub maker: AccountId,

    // TODO: check != src_asset
    // TODO: support one_of for dst
    pub src_token: TokenId,
    pub dst_token: TokenId,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub refund_src_to: OverrideSend,
    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub receive_dst_to: OverrideSend,

    #[serde(default)]
    pub partial_fills_allowed: bool,

    // TODO: check that fees are non-zero
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fees: BTreeMap<AccountId, Pips>,

    // TODO: store only merkle root? leaves have salts
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub taker_whitelist: BTreeSet<AccountId>,
    // taker_whitelist: ["solver-bus-proxy.near"] (knows SolverBus public key)

    // solver -> intents.near::mt_transfer_call():
    //   * solver-bus-proxy.near::mt_on_transfer(sender_id, token_id, amount, msg):
    //      msg.extract_solver_bus_signature()
    //               verify_signature()
    //               if ok -> forward transfer to escrow specified in msg
    //               if not ok -> refund solver
    //
    // solver-bus.near -> solver-bus-proxy.near::close(escrow_contract_id)
    //                 -> escrow-0x1234....abc::close()
    //

    //

    // TODO: whitelist: Option<signer_id>

    // TODO: or parent account id?
    #[cfg(feature = "auth_call")] // TODO: borsh order?
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_caller: Option<AccountId>,

    #[serde_as(as = "Hex")]
    pub salt: [u8; 4],
    // TODO: authority
    // TODO: close authority: intents adapter for on_auth

    // TODO: taker-change authority should be implemented as taker-gateway, which keeps taker whitelist in runtime

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
            .values()
            .copied()
            .try_fold(Pips::ZERO, Pips::checked_add)
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Default, Clone, PartialEq, Eq)]
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

    #[serde(skip)] // callers shouldn't care
    pub callbacks_in_flight: u32,
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverrideSend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_id: Option<AccountId>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl OverrideSend {
    pub fn verify(&self) -> Result<()> {
        // TODO: verify min_gas < MAX_GAS
        Ok(())
    }
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
