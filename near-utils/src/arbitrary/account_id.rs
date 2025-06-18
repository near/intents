use std::iter;

use arbitrary_with::{Arbitrary, ArbitraryAs, Error, Result, Unstructured, UnstructuredExt};
use near_account_id::AccountType;
use near_sdk::{AccountId, AccountIdRef};

const MAX_ACCOUNT_ID_LENGTH: usize = 64;

pub struct ArbitraryAccountId;

impl<'a> ArbitraryAs<'a, AccountId> for ArbitraryAccountId {
    fn arbitrary_as(u: &mut Unstructured<'a>) -> Result<AccountId> {
        match u.choose(&[
            AccountType::NearImplicitAccount,
            AccountType::EthImplicitAccount,
            AccountType::NamedAccount,
        ])? {
            AccountType::NamedAccount => u.arbitrary_as::<_, ArbitraryNamedAccountId>(),
            AccountType::NearImplicitAccount => {
                u.arbitrary_as::<_, ArbitraryImplicitNearAccountId>()
            }
            AccountType::EthImplicitAccount => u.arbitrary_as::<_, ArbitraryImplicitEthAccountId>(),
        }
    }
}

pub struct ArbitraryImplicitNearAccountId;

impl<'a> ArbitraryAs<'a, AccountId> for ArbitraryImplicitNearAccountId {
    fn arbitrary_as(u: &mut Unstructured<'a>) -> Result<AccountId> {
        hex::encode(<[u8; 32]>::arbitrary(u)?)
            .parse()
            .map_err(|_| Error::IncorrectFormat)
    }
}

pub struct ArbitraryImplicitEthAccountId;

impl<'a> ArbitraryAs<'a, AccountId> for ArbitraryImplicitEthAccountId {
    fn arbitrary_as(u: &mut Unstructured<'a>) -> Result<AccountId> {
        format!("0x{}", hex::encode(<[u8; 20]>::arbitrary(u)?))
            .parse()
            .map_err(|_| Error::IncorrectFormat)
    }
}

pub struct ArbitraryNamedAccountId;

impl ArbitraryNamedAccountId {
    const NON_EDGE_ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz-_";
    const EDGE_ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

    fn char(u: &mut Unstructured<'_>, on_edge: bool) -> Result<char> {
        u.choose(if on_edge {
            Self::EDGE_ALPHABET
        } else {
            Self::NON_EDGE_ALPHABET
        })
        .map(|c| (*c).into())
    }

    pub fn arbitrary_subaccount(
        u: &mut Unstructured<'_>,
        parent: Option<&AccountIdRef>,
    ) -> Result<AccountId> {
        let len_bounds = parent.map_or(
            // TLA
            2..=MAX_ACCOUNT_ID_LENGTH,
            #[allow(clippy::range_minus_one)]
            |parent| 1..=MAX_ACCOUNT_ID_LENGTH - parent.len() - 1,
        );

        let len = u
            .int_in_range(len_bounds)?
            // subaccount can't be empty
            .max(1);

        // account_id can't start with '-' or '_'
        let first = Self::char(u, true)?;

        let subaccount: String = if len == 1 {
            first.into()
        } else {
            let last = Self::char(u, true)?;

            iter::once(Ok(first))
                .chain(
                    iter::repeat_with({
                        // '-' and '_' must be followed by edge char
                        let mut last_not_edge = false;
                        move || {
                            Self::char(u, last_not_edge)
                                .inspect(|c| last_not_edge = ['-', '_'].contains(c))
                        }
                    })
                    .take(len - 2),
                )
                .chain(iter::once(Ok(last)))
                .collect::<Result<_>>()?
        };

        if let Some(parent) = parent {
            format!("{subaccount}.{parent}")
        } else {
            subaccount
        }
        .parse()
        .map_err(|_| Error::IncorrectFormat)
    }
}

impl<'a> ArbitraryAs<'a, AccountId> for ArbitraryNamedAccountId {
    fn arbitrary_as(u: &mut Unstructured<'a>) -> Result<AccountId> {
        // TLA
        let mut account_id = Self::arbitrary_subaccount(u, None).unwrap();

        // keep adding subaccounts while there is enough space for at least
        // single character + '.'
        while account_id.len() < MAX_ACCOUNT_ID_LENGTH - 2 && u.arbitrary()? {
            account_id = Self::arbitrary_subaccount(u, Some(&account_id))?;
        }
        Ok(account_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use arbitrary_with::{Unstructured, UnstructuredExt};
    use near_account_id::AccountType;
    use rstest::rstest;

    use defuse_test_utils::random::random_bytes;

    #[rstest]
    fn basic(#[with(..1000000)] random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);

        for _ in 0..10 {
            {
                assert!(matches!(
                    u.arbitrary_as::<_, ArbitraryImplicitNearAccountId>()
                        .unwrap()
                        .get_account_type(),
                    AccountType::NearImplicitAccount
                ));
            }
            {
                assert!(matches!(
                    u.arbitrary_as::<_, ArbitraryImplicitEthAccountId>()
                        .unwrap()
                        .get_account_type(),
                    AccountType::EthImplicitAccount
                ));
            }
            {
                assert!(matches!(
                    u.arbitrary_as::<_, ArbitraryNamedAccountId>()
                        .unwrap()
                        .get_account_type(),
                    AccountType::NamedAccount
                ));
            }
        }
    }

    #[rstest]
    fn named_tla(random_bytes: Vec<u8>) {
        let mut u = Unstructured::new(&random_bytes);
        assert!(
            ArbitraryNamedAccountId::arbitrary_subaccount(&mut u, None)
                .unwrap()
                .is_top_level()
        );
    }

    #[rstest]
    fn named_subaccount(random_bytes: Vec<u8>) {
        const TLA: &AccountIdRef = AccountIdRef::new_or_panic("near");

        let mut u = Unstructured::new(&random_bytes);

        assert!(
            ArbitraryNamedAccountId::arbitrary_subaccount(&mut u, Some(TLA))
                .unwrap()
                .is_sub_account_of(TLA)
        );
    }
}
