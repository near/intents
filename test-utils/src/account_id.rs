use arbitrary::Unstructured;
use near_sdk::AccountId;

#[allow(clippy::as_conversions)]
pub fn arbitrary_account_id(u: &mut Unstructured<'_>) -> arbitrary::Result<AccountId> {
    if u.arbitrary()? {
        // Named account id
        let len = u.int_in_range(3..=20)?;
        let s: String = (0..len)
            .map(|_| {
                let c = u.int_in_range(0..=35)?;
                Ok(match c {
                    0..=25 => (b'a' + c) as char,
                    26..=35 => (b'0' + (c - 26)) as char,
                    _ => unreachable!(),
                })
            })
            .collect::<arbitrary::Result<_>>()?;
        let s = s + ".near";
        s.parse().map_err(|_| arbitrary::Error::IncorrectFormat)
    } else {
        // Explicit numeric account id
        let len = 64;
        let s: String = (0..len)
            .map(|_| {
                let c = u.int_in_range(0..=15)?;
                Ok(match c {
                    0..=9 => (b'0' + c) as char,
                    10..=15 => (b'a' + (c - 10)) as char,
                    _ => unreachable!(),
                })
            })
            .collect::<arbitrary::Result<_>>()?;
        s.parse().map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}
