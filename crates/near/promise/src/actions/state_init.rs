use near_gas::NearGas as Gas;
use near_global_contracts::{StateInit, StateInitV1};
use near_token::NearToken;

/// `DeterministicStateInit` [action](crate::actions::NearAction)
/// as per [NEP-616](https://github.com/near/NEPs/blob/master/neps/nep-0616.md)
#[must_use = "promises do nothing unless you `.build()` them"]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeterministicStateInit {
    pub state_init: StateInit,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "NearToken::is_zero")
    )]
    pub deposit: NearToken,
}

impl DeterministicStateInit {
    #[inline]
    pub fn new(state_init: impl Into<StateInit>) -> Self {
        Self {
            state_init: state_init.into(),
            deposit: NearToken::ZERO,
        }
    }

    #[inline]
    pub const fn deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }

    pub(crate) fn estimate_gas(&self) -> Gas {
        const STATE_INIT_BASE: Gas = Gas::from_tgas(9);
        const STATE_INIT_PER_ENTRY: Gas = Gas::from_ggas(210);
        const STATE_INIT_PER_BYTE: Gas = Gas::from_gas(160_000_000);

        let StateInit::V1(state_init) = &self.state_init;

        STATE_INIT_BASE
            .saturating_add(
                STATE_INIT_PER_ENTRY
                    .saturating_mul(state_init.data.len().try_into().unwrap_or(u64::MAX)),
            )
            .saturating_add(
                STATE_INIT_PER_BYTE.saturating_mul(
                    state_init
                        .data
                        .iter()
                        .map(|(k, v)| k.len().saturating_add(v.len()))
                        .fold(0usize, usize::saturating_add)
                        .try_into()
                        .unwrap_or(u64::MAX),
                ),
            )
    }
}

impl From<StateInit> for DeterministicStateInit {
    #[inline]
    fn from(state_init: StateInit) -> Self {
        Self::new(state_init)
    }
}

impl From<StateInitV1> for DeterministicStateInit {
    #[inline]
    fn from(state_init: StateInitV1) -> Self {
        StateInit::from(state_init).into()
    }
}

#[cfg(feature = "near-kit")]
const _: () = {
    use near_kit::DeterministicStateInitAction;

    impl From<DeterministicStateInitAction> for DeterministicStateInit {
        #[inline]
        fn from(value: DeterministicStateInitAction) -> Self {
            Self {
                state_init: value.state_init,
                deposit: value.deposit,
            }
        }
    }

    impl From<DeterministicStateInit> for DeterministicStateInitAction {
        #[inline]
        fn from(value: DeterministicStateInit) -> Self {
            Self {
                state_init: value.state_init,
                deposit: value.deposit,
            }
        }
    }
};

#[cfg(test)]
mod tests {

    use near_global_contracts::{GlobalContractId, StateInitV1};

    use super::*;

    #[test]
    fn estimate_gas_zba() {
        let estimate = DeterministicStateInit {
            state_init: StateInit::V1(StateInitV1 {
                code: GlobalContractId::AccountId(
                    "longlonglonglonglonglonglonglonglonglonglonglonglonglonglonglong"
                        .parse()
                        .unwrap(),
                ),
                data: (0..=16).map(|i| (vec![i], vec![])).collect(),
            }),
            deposit: NearToken::from_near(1),
        }
        .estimate_gas();

        assert!(
            estimate <= Gas::from_tgas(15),
            "DeterministicStateInit for deterministic account within ZBA limits should be no greater than 15 TGas",
        );
    }
}
