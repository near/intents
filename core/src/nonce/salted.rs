use crate::{DefuseError, Result};
use core::mem;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near,
};

pub type Salt = [u8; 4];

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidSalts {
    previous: Option<Salt>,
    current: Salt,
}

impl ValidSalts {
    /// There can be only one valid salt at the beginning
    #[inline]
    pub fn new(salt: Salt) -> Self {
        Self {
            previous: None,
            current: salt,
        }
    }

    #[inline]
    pub fn contains_salt(&self, salt: Salt) -> bool {
        salt == self.current || self.previous.is_some_and(|s| s == salt)
    }

    #[inline]
    pub fn rotate_salt(&mut self, new_salt: &mut Salt) -> Result<ValidSalts> {
        if self.contains_salt(*new_salt) {
            return Err(DefuseError::InvalidSalt);
        }

        let old_salts = self.clone();

        let legacy_salt = mem::replace(&mut self.current, *new_salt);
        self.previous = Some(legacy_salt);

        Ok(old_salts)
    }

    #[inline]
    pub fn reset_salts(&mut self, new_salts: ValidSalts) -> ValidSalts {
        // TODO: do we need to check if they are different?
        mem::replace(self, new_salts)
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

    use defuse_test_utils::random::random_bytes;
    use hex_literal::hex;
    use rstest::rstest;

    #[rstest]
    fn contains_salt_test(random_bytes: Vec<u8>) {
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let salts: ValidSalts = u.arbitrary().unwrap();
        let random_salt: [u8; 4] = u.arbitrary().unwrap();

        assert!(salts.contains_salt(salts.current));
        assert!(!salts.contains_salt(random_salt));
    }

    #[rstest]
    fn rotate_salt_test(random_bytes: Vec<u8>) {
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let mut salts: ValidSalts = u.arbitrary().unwrap();
        let old_salts = salts.clone();
        let mut random_salt: [u8; 4] = u.arbitrary().unwrap();

        let success = salts.rotate_salt(&mut random_salt);

        assert!(success.is_ok());
        assert_eq!(success.unwrap(), old_salts);
        assert!(salts.contains_salt(old_salts.current));
        assert!(salts.contains_salt(random_salt));

        let fail = salts.rotate_salt(&mut random_salt);
        assert!(fail.is_err());
    }

    #[rstest]
    fn reset_salt_test(random_bytes: Vec<u8>) {
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let mut salts: ValidSalts = u.arbitrary().unwrap();

        let new_salts: ValidSalts = u.arbitrary().unwrap();
        let old_salts = salts.clone();

        let replaced = salts.reset_salts(new_salts.clone());

        assert_eq!(salts, new_salts);
        assert_eq!(replaced, old_salts);
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
