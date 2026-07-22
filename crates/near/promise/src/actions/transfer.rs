use near_token::NearToken;

/// Transfer [action](crate::actions::NearAction).
#[must_use = "promises do nothing unless you `.build()` them"]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Transfer {
    /// Amount of NEAR tokens to transfer
    pub amount: NearToken,
}

impl From<NearToken> for Transfer {
    #[inline]
    fn from(amount: NearToken) -> Self {
        Self { amount }
    }
}

#[cfg(feature = "near-kit")]
const _: () = {
    use near_kit::TransferAction;

    impl From<TransferAction> for Transfer {
        #[inline]
        fn from(value: TransferAction) -> Self {
            Self {
                amount: value.deposit,
            }
        }
    }

    impl From<Transfer> for TransferAction {
        #[inline]
        fn from(value: Transfer) -> Self {
            Self {
                deposit: value.amount,
            }
        }
    }
};
