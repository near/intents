use core::mem;
use impl_tools::autoimpl;
use near_sdk::{
    IntoStorageKey,
    borsh::{BorshDeserialize, BorshSerialize},
    env, near,
    store::IterableMap,
};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Copy, Clone)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[autoimpl(AsRef using self.0)]
#[autoimpl(AsMut using self.0)]
#[near(serializers = [borsh, json])]
pub struct Salt([u8; 4]);

impl Salt {
    fn random() -> Self {
        const SIZE: usize = size_of::<Salt>();
        let seed = &env::random_seed_array()[..SIZE];
        let mut result = [0u8; SIZE];

        result.copy_from_slice(seed);

        Self(result)
    }
}

impl From<&[u8]> for Salt {
    fn from(value: &[u8]) -> Self {
        let mut result = [0u8; 4];
        result.copy_from_slice(&value[..4]);
        Self(result)
    }
}

/// Contains current valid salt and set of previous
/// salts that can be valid or invalid.
#[near(serializers = [borsh])]
#[derive(Debug)]
pub struct ValidSalts {
    previous: IterableMap<Salt, bool>,
    current: Salt,
}

impl ValidSalts {
    /// There can be only one valid salt at the beginning
    #[inline]
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self {
            previous: IterableMap::new(prefix),
            current: Salt::random(),
        }
    }

    /// Rotates the current salt, making the previous salt valid as well.
    #[inline]
    pub fn set_new(&mut self, invalidate_current: bool) -> Salt {
        let salt = Salt::random();
        let previous = mem::replace(&mut self.current, salt);

        self.previous.insert(previous, !invalidate_current);

        previous
    }

    /// Deactivates the previous salt, making it invalid.
    #[inline]
    pub fn invalidate(&mut self, salt: Salt) -> bool {
        if salt == self.current {
            self.set_new(true);

            return true;
        }

        self.previous.get_mut(&salt).map(|v| *v = false).is_some()
    }

    #[inline]
    pub fn is_valid(&self, salt: Salt) -> bool {
        salt == self.current || self.previous.get(&salt).is_some_and(|v| *v)
    }

    #[inline]
    pub const fn current(&self) -> Salt {
        self.current
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
pub struct SaltedNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub salt: Salt,
    pub nonce: T,
}

impl<T> SaltedNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub const fn new(salt: Salt, nonce: T) -> Self {
        Self { salt, nonce }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use arbitrary::Unstructured;
    use defuse_test_utils::random::{Rng, random_bytes, rng};
    use near_sdk::{test_utils::VMContextBuilder, testing_env};

    use rstest::rstest;

    fn set_random_seed(rng: &mut impl Rng) -> Salt {
        let seed = rng.random();
        let context = VMContextBuilder::new().random_seed(seed).build();
        testing_env!(context);

        seed[..4].into()
    }

    #[rstest]
    fn contains_salt_test(random_bytes: Vec<u8>) {
        let random_salt: Salt = Unstructured::new(&random_bytes).arbitrary().unwrap();
        let salts = ValidSalts::new(random_bytes);

        assert!(salts.is_valid(salts.current));
        assert!(!salts.is_valid(random_salt));
    }

    #[rstest]
    fn rotate_salt_test(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let mut salts = ValidSalts::new(random_bytes);

        let current = set_random_seed(&mut rng);
        let previous = salts.set_new(false);

        assert!(salts.is_valid(current));
        assert!(salts.is_valid(previous));

        let current = set_random_seed(&mut rng);
        let previous = salts.set_new(true);

        assert!(salts.is_valid(current));
        assert!(!salts.is_valid(previous));
    }

    #[rstest]
    fn reset_salt_test(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let mut salts = ValidSalts::new(random_bytes);
        let random_salt = rng.random::<[u8; 4]>().as_slice().into();

        let current = set_random_seed(&mut rng);
        let previous = salts.set_new(false);

        assert!(salts.invalidate(previous));
        assert!(!salts.is_valid(previous));
        assert!(!salts.invalidate(random_salt));

        let new_salt = set_random_seed(&mut rng);
        assert!(salts.invalidate(current));
        assert!(!salts.is_valid(current));
        assert_eq!(salts.current(), new_salt);
    }
}
