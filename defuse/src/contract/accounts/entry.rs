use std::{
    borrow::Cow,
    io::{self, Read},
};

use defuse_borsh_utils::adapters::{As, BorshDeserializeAs, BorshSerializeAs};
use defuse_io_utils::ReadExt;
use defuse_near_utils::{Lock, PanicOnClone};
use impl_tools::autoimpl;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near,
};

use super::Account;

#[derive(Debug)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[autoimpl(AsRef using self.0)]
#[autoimpl(AsMut using self.0)]
#[near(serializers = [borsh])]
#[repr(transparent)]
pub struct AccountEntry(
    #[borsh(
        deserialize_with = "As::<MaybeVersionedAccountEntry>::deserialize",
        serialize_with = "As::<MaybeVersionedAccountEntry>::serialize"
    )]
    pub Lock<Account>,
);

impl From<Lock<Account>> for AccountEntry {
    #[inline]
    fn from(value: Lock<Account>) -> Self {
        Self(value)
    }
}

/// This is a magic number that is used to differentiate between
/// borsh-serialized representations of legacy and versioned [`Account`]s:
/// * versioned [`Account`]s always start with this prefix
/// * legacy [`Account`] starts with other 4 bytes
///
/// This is safe to assume that legacy [`Account`] doesn't start with
/// this prefix, since the first 4 bytes in legacy [`Account`] were used
/// to denote the length of `prefix: Box<[u8]>` in [`LookupMap`] for
/// `nonces`. Given that the original prefix is reused for other fields of
/// [`Account`] for creating other nested prefixes, then the length of
/// this prefix can't be the maximum of what `Box<[u8]>` can be
/// serialized to.
const VERSIONED_MAGIC_PREFIX: u32 = u32::MAX;

/// Versioned [Account] state for de/serialization
#[derive(Debug)]
#[near(serializers = [borsh])]
enum VersionedAccountEntry<'a> {
    V0(Cow<'a, PanicOnClone<Account>>),
    V1(Cow<'a, PanicOnClone<Lock<Account>>>),
}

impl From<Account> for VersionedAccountEntry<'_> {
    fn from(value: Account) -> Self {
        Self::V0(Cow::Owned(value.into()))
    }
}

impl<'a> From<&'a Lock<Account>> for VersionedAccountEntry<'a> {
    fn from(value: &'a Lock<Account>) -> Self {
        // always serialize as latest version
        Self::V1(Cow::Borrowed(PanicOnClone::from_ref(value)))
    }
}

impl From<VersionedAccountEntry<'_>> for Lock<Account> {
    fn from(versioned: VersionedAccountEntry<'_>) -> Self {
        // Borsh always deserializes into `Cow::Owned`, so it's
        // safe to call `Cow::<PanicOnClone<_>>::into_owned()` here.
        match versioned {
            VersionedAccountEntry::V0(account) => account.into_owned().into_inner().into(),
            VersionedAccountEntry::V1(account) => account.into_owned().into_inner(),
        }
    }
}

struct MaybeVersionedAccountEntry;

impl BorshDeserializeAs<Lock<Account>> for MaybeVersionedAccountEntry {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Lock<Account>>
    where
        R: io::Read,
    {
        let mut buf = Vec::new();
        // There will always be 4 bytes for u32:
        // * either `VERSIONED_MAGIC_PREFIX`,
        // * or u32 for `Account.nonces.prefix`
        let prefix = u32::deserialize_reader(&mut reader.tee(&mut buf))?;

        if prefix == VERSIONED_MAGIC_PREFIX {
            VersionedAccountEntry::deserialize_reader(reader)
        } else {
            Account::deserialize_reader(
                // prepend already consumed part of the reader
                &mut buf.chain(reader),
            )
            .map(Into::into)
        }
        .map(Into::into)
    }
}

impl BorshSerializeAs<Lock<Account>> for MaybeVersionedAccountEntry {
    fn serialize_as<W>(source: &Lock<Account>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        (
            // always serialize as versioned and prepend magic prefix
            VERSIONED_MAGIC_PREFIX,
            VersionedAccountEntry::from(source),
        )
            .serialize(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::{HashMap, HashSet};

    use arbitrary_with::{Arbitrary, As, arbitrary};
    use defuse_bitmap::U256;
    use defuse_core::{crypto::PublicKey, token_id::TokenId};
    use defuse_near_utils::arbitrary::ArbitraryAccountId;
    use defuse_test_utils::random::make_arbitrary;

    use near_sdk::{AccountId, borsh};
    use rstest::rstest;

    #[derive(Arbitrary)]
    struct AccountData {
        prefix: Vec<u8>,
        #[arbitrary(with = As::<ArbitraryAccountId>::arbitrary)]
        account_id: AccountId,

        public_keys: HashSet<PublicKey>,
        nonces: HashSet<U256>,
        token_balances: HashMap<TokenId, u128>,
    }

    impl AccountData {
        fn make_account(&self) -> Account {
            let mut a = Account::new(self.prefix.as_slice(), &self.account_id);

            for &pk in &self.public_keys {
                assert!(a.add_public_key(&self.account_id, pk));
            }

            for &n in &self.nonces {
                assert!(a.commit_nonce(n));
            }

            for (token_id, &amount) in &self.token_balances {
                assert!(a.token_balances.add(token_id.clone(), amount).is_some());
            }

            a
        }

        fn assert_contained_in(&self, a: &Account) {
            for pk in &self.public_keys {
                assert!(a.has_public_key(&self.account_id, pk));
            }

            for &n in &self.nonces {
                assert!(a.is_nonce_used(n));
            }

            for (token_id, &amount) in &self.token_balances {
                assert_eq!(a.token_balances.amount_for(token_id), amount);
            }
        }
    }

    #[rstest]
    fn legacy_upgrade(#[from(make_arbitrary)] data: AccountData) {
        let serialized_legacy = {
            let legacy = data.make_account();
            borsh::to_vec(&legacy).expect("unable to serialize legacy Account")
        };

        let serialized_versioned = {
            let mut versioned: AccountEntry = borsh::from_slice(&serialized_legacy).unwrap();
            data.assert_contained_in(
                versioned
                    .lock()
                    .expect("legacy accounts must be unlocked by default"),
            );
            borsh::to_vec(&versioned).unwrap()
        };

        {
            let versioned: AccountEntry = borsh::from_slice(&serialized_versioned).unwrap();
            data.assert_contained_in(versioned.as_locked().expect("should be locked by now"));
        }
    }
}
