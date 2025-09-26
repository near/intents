use core::mem;
use near_sdk::{
    IntoStorageKey,
    borsh::{BorshDeserialize, BorshSerialize},
    env, near,
    store::IterableMap,
};

pub type Salt = [u8; 4];

#[near(serializers = [borsh])]
#[derive(Debug)]
pub struct ValidSalts {
    previous: IterableMap<Salt, bool>,
    current: Salt,
}

impl ValidSalts {
    fn get_random_salt() -> Salt {
        // NOTE: it is safe to unwrap here as random_seed_array always returns 32 bytes
        env::random_seed_array()[..4].try_into().unwrap()
    }

    /// There can be only one valid salt at the beginning
    #[inline]
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self {
            previous: IterableMap::new(prefix),
            current: Self::get_random_salt(),
        }
    }

    #[inline]
    pub fn set_new(&mut self) -> Salt {
        let salt = Self::get_random_salt();
        let previous = mem::replace(&mut self.current, salt);

        self.previous.insert(previous, true);

        previous
    }

    #[inline]
    pub fn clear_previous(&mut self, salt: &Salt) -> bool {
        self.previous.get_mut(salt).map(|v| *v = false).is_some()
    }

    #[inline]
    pub fn is_valid(&self, salt: &Salt) -> bool {
        salt == &self.current || self.previous.get(salt).is_some_and(|v| *v == true)
    }

    #[inline]
    pub fn current(&self) -> &Salt {
        &self.current
    }
}

/// Salted nonces contain 4 bytes salt from a predefined set of salts
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

    pub fn is_valid_salt(&self, expected_salts: &[Salt]) -> bool {
        expected_salts.iter().any(|s| s == &self.salt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use arbitrary::Unstructured;
    use defuse_test_utils::random::{Rng, random_bytes, rng};
    use hex_literal::hex;
    use near_sdk::{test_utils::VMContextBuilder, testing_env};

    use rstest::rstest;

    fn set_random_seed(mut rng: impl Rng) -> Salt {
        let seed = rng.random();
        let context = VMContextBuilder::new().random_seed(seed).build();
        testing_env!(context);

        seed[..4].try_into().unwrap()
    }

    #[rstest]
    fn contains_salt_test(random_bytes: Vec<u8>) {
        let random_salt: Salt = Unstructured::new(&random_bytes).arbitrary().unwrap();
        let salts = ValidSalts::new(random_bytes);

        assert!(salts.is_valid(&salts.current));
        assert!(!salts.is_valid(&random_salt));
    }

    #[rstest]
    fn rotate_salt_test(random_bytes: Vec<u8>, rng: impl Rng) {
        let mut salts = ValidSalts::new(random_bytes);

        let current = set_random_seed(rng);
        let previous = salts.set_new();

        assert!(salts.is_valid(&current));
        assert!(salts.is_valid(&previous));
    }

    #[rstest]
    fn reset_salt_test(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let mut salts = ValidSalts::new(random_bytes);
        let random_salt: Salt = rng.random();

        let current = set_random_seed(rng);
        let previous = salts.set_new();

        assert!(salts.clear_previous(&previous));
        assert!(!salts.is_valid(&previous));
        assert!(!salts.clear_previous(&random_salt));
        assert!(!salts.clear_previous(&current));
    }

    #[rstest]
    fn valid_salted_nonce_test(random_bytes: Vec<u8>) {
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let nonce: [u8; 28] = u.arbitrary().unwrap();

        let salts = [hex!("00010203"), hex!("04050607"), hex!("08090a0b")];

        let valid_salted_nonce = SaltedNonce::new(salts[1], nonce);
        assert!(valid_salted_nonce.is_valid_salt(&salts));

        let invalid_salted_nonce = SaltedNonce::new(hex!("8d969eef"), nonce);
        assert!(!invalid_salted_nonce.is_valid_salt(&salts));
    }
}
