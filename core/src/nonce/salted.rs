use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

pub type Salt = [u8; 4];

/// Salted nonces contain 4 bytes salt from a predefined set of salts
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
pub struct SaltedNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    salt: Salt,
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
    fn valid_salt_test(random_bytes: Vec<u8>) {
        let mut u = arbitrary::Unstructured::new(&random_bytes);
        let nonce: [u8; 28] = u.arbitrary().unwrap();

        let salts = [hex!("00010203"), hex!("04050607"), hex!("08090a0b")];

        let valid_salted_nonce = SaltedNonce::new(salts[1], nonce);
        assert!(valid_salted_nonce.is_valid_salt(&salts));

        let invalid_salted_nonce = SaltedNonce::new(hex!("8d969eef"), nonce);
        assert!(!invalid_salted_nonce.is_valid_salt(&salts));
    }
}
