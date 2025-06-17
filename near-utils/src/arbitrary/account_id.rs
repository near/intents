use arbitrary_with::{Arbitrary, ArbitraryAs, Error, Result, Unstructured, UnstructuredExt};
use near_account_id::AccountType;
use near_sdk::AccountId;

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

impl<'a> ArbitraryAs<'a, AccountId> for ArbitraryNamedAccountId {
    fn arbitrary_as(u: &mut Unstructured<'a>) -> Result<AccountId> {
        let make_subaccount = |account_id: &str, u: &mut Unstructured<'a>| -> Result<String> {
            let subaccount_len =
                u.int_in_range(2..=(MAX_ACCOUNT_ID_LENGTH - account_id.len() - 1))?;
            (0..subaccount_len)
                .map(|_| {
                    let c = u.int_in_range(0..=35)?;
                    Ok(match c {
                        0..=25 => (b'a' + c) as char,
                        26..=35 => (b'0' + (c - 26)) as char,
                        _ => unreachable!(),
                    })
                })
                .collect::<Result<_>>()
        };

        let mut account_id = make_subaccount("", u)?;

        while account_id.len() < MAX_ACCOUNT_ID_LENGTH - 2 && u.arbitrary()? {
            account_id = [make_subaccount(&account_id, u)?, account_id].join(".");
        }
        account_id.parse().map_err(|_| Error::IncorrectFormat)
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
}
