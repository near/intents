use crate::arbitrary::hex::arbitrary_hex_fixed_size;
use arbitrary::{Arbitrary, Unstructured};
use near_account_id::AccountType;
use near_sdk::AccountId;

const MAX_NEAR_ACCOUNT_LENGTH: usize = 64;

pub fn arbitrary_account_id(u: &mut Unstructured<'_>) -> arbitrary::Result<AccountId> {
    let types = [
        AccountType::NamedAccount,
        AccountType::NearImplicitAccount,
        AccountType::EthImplicitAccount,
    ];
    let choice = u.choose(&types)?;

    match choice {
        AccountType::NamedAccount => arbitrary_near_named_account_id(u),
        AccountType::NearImplicitAccount => arbitrary_near_implicit_account_id(u),
        AccountType::EthImplicitAccount => arbitrary_ethereum_account_id(u),
    }
}

pub fn arbitrary_ethereum_account_id(u: &mut Unstructured<'_>) -> arbitrary::Result<AccountId> {
    let s = arbitrary_hex_fixed_size::<20>(u)?;
    format!("0x{s}")
        .parse()
        .map_err(|_| arbitrary::Error::IncorrectFormat)
}

pub fn arbitrary_near_implicit_account_id(
    u: &mut Unstructured<'_>,
) -> arbitrary::Result<AccountId> {
    let s = arbitrary_hex_fixed_size::<32>(u)?;
    s.parse().map_err(|_| arbitrary::Error::IncorrectFormat)
}

#[allow(clippy::as_conversions)]
fn arbitrary_near_named_account_prefix(u: &mut Unstructured<'_>) -> arbitrary::Result<String> {
    let len = u.int_in_range(3..=40)?;
    (0..len)
        .map(|_| {
            let c = u.int_in_range(0..=35)?;
            Ok(match c {
                0..=25 => (b'a' + c) as char,
                26..=35 => (b'0' + (c - 26)) as char,
                _ => unreachable!(),
            })
        })
        .collect::<arbitrary::Result<_>>()
}

fn arbitrary_near_named_account_tla(u: &mut Unstructured<'_>) -> arbitrary::Result<String> {
    let possibilities = ["near"];
    let index = usize::arbitrary(u)? % possibilities.len();
    Ok(possibilities[index].to_string())
}

pub fn arbitrary_near_named_account_id(u: &mut Unstructured<'_>) -> arbitrary::Result<AccountId> {
    let sub_account_count = u.int_in_range(1..=20)?;
    let account_subs = (0..sub_account_count)
        .map(|_| arbitrary_near_named_account_prefix(u))
        .collect::<arbitrary::Result<Vec<String>>>()?;
    let account_tla = arbitrary_near_named_account_tla(u)?;

    // This is the full account + TLA
    let mut account_parts = account_subs
        .into_iter()
        .chain(std::iter::once(account_tla))
        .collect::<Vec<_>>();

    let get_total_length = |parts: &[String]| -> usize {
        parts.iter().map(String::len).sum::<usize>() + parts.len() - 1
    };

    // Keep chopping sub accounts from the front
    // until the size is not more than the max
    while get_total_length(&account_parts) > MAX_NEAR_ACCOUNT_LENGTH {
        account_parts = account_parts.into_iter().skip(1).collect();
    }

    assert!(account_parts.len() >= 2);

    let account = account_parts.join(".");
    account
        .parse()
        .map_err(|_| arbitrary::Error::IncorrectFormat)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{
        arbitrary::account_id::{
            arbitrary_ethereum_account_id, arbitrary_near_implicit_account_id,
            arbitrary_near_named_account_id,
        },
        random::{Seed, gen_random_bytes, make_seedable_rng, random_seed},
    };

    #[rstest]
    #[trace]
    fn basic(random_seed: Seed) {
        let mut rng = make_seedable_rng(random_seed);
        let bytes = gen_random_bytes(&mut rng, ..1000000);
        let mut u = arbitrary::Unstructured::new(&bytes);

        for _ in 0..10 {
            {
                let account = arbitrary_near_implicit_account_id(&mut u).unwrap();
                assert_eq!(account.len(), 64);
            }
            {
                let account = arbitrary_near_named_account_id(&mut u).unwrap();
                assert!(account.to_string().contains('.'));
            }
            {
                let account = arbitrary_ethereum_account_id(&mut u).unwrap();
                assert_eq!(account.to_string().len(), 42);
            }
        }
    }
}
