use near_account_id::AccountId;

use crate::Gas;

/// Gas reserved for each token `ft_metadata()` call made for
/// [`WalletOp::RequireFtMetadata`].
pub const FT_METADATA_GAS: Gas = Gas::from_tgas(10);

/// Gas reserved for each wallet callback that checks `ft_metadata()`, in
/// addition to the gas of the promise it creates.
pub const FT_METADATA_CALLBACK_GAS: Gas = Gas::from_tgas(10);

/// An internal operation to perform on wallet-contract's [state](crate::State).
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(
        tag = "op",
        content = "payload",
        rename_all = "snake_case",
        deny_unknown_fields
    )
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WalletOp {
    /// Enable or disable authentication by signature.
    ///
    /// If signature mode is already in specified state, the contract MUST panic.
    SetSignatureMode { enable: bool } = 0,

    /// Add an extension with given `AccountId`.
    ///
    /// If this extension is already enabled, the contract MUST panic.
    AddExtension { account_id: AccountId } = 1,

    /// Remove an extension with given `AccountId`.
    ///
    /// If this extension is not currently enabled, the contract MUST panic.
    RemoveExtension { account_id: AccountId } = 2,

    /// Create this request's promises to `token_id` only after its NEP-148
    /// `ft_metadata()` returns exactly these `decimals` and `symbol`;
    /// otherwise they are not created.
    ///
    /// Readable signing messages require it to show NEP-141 transfer amounts
    /// with decimals and symbol. At most one per token in a request.
    RequireFtMetadata {
        token_id: AccountId,
        decimals: u8,
        symbol: String,
    } = 3,
}

impl WalletOp {
    #[inline]
    pub const fn set_signature_mode(enable: bool) -> Self {
        Self::SetSignatureMode { enable }
    }

    #[inline]
    pub const fn enable_signature() -> Self {
        Self::set_signature_mode(true)
    }

    #[inline]
    pub const fn disable_signature() -> Self {
        Self::set_signature_mode(false)
    }

    #[inline]
    pub fn add_extension(account_id: impl Into<AccountId>) -> Self {
        Self::AddExtension {
            account_id: account_id.into(),
        }
    }

    #[inline]
    pub fn remove_extension(account_id: impl Into<AccountId>) -> Self {
        Self::RemoveExtension {
            account_id: account_id.into(),
        }
    }

    #[inline]
    pub fn require_ft_metadata(
        token_id: impl Into<AccountId>,
        decimals: u8,
        symbol: impl Into<String>,
    ) -> Self {
        Self::RequireFtMetadata {
            token_id: token_id.into(),
            decimals,
            symbol: symbol.into(),
        }
    }
}
