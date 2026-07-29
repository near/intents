use borsh::{BorshDeserialize, BorshSerialize};
use core::mem;
use defuse_digest::{Digest, sha2::Sha256};
use defuse_map_utils::Map;
use hex::FromHex;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{
    fmt::{self, Debug},
    str::FromStr,
};

use crate::{DefuseError, Result};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "abi", derive(::borsh::BorshSchema))]
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    SerializeDisplay,
    DeserializeFromStr,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct Salt([u8; 4]);

impl Salt {
    pub fn derive(num: u8, seed: impl Into<[u8; 32]>) -> Self {
        const SIZE: usize = size_of::<Salt>();

        let seed = seed.into();
        let mut input = [0u8; 33];
        input[..32].copy_from_slice(&seed);
        input[32] = num;

        Self(
            Sha256::digest(input)[..SIZE]
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
        )
    }
}

impl fmt::Debug for Salt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl fmt::Display for Salt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl FromStr for Salt {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        FromHex::from_hex(s).map(Self)
    }
}

#[cfg(feature = "abi")]
const _: () = {
    use schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject},
    };

    impl JsonSchema for Salt {
        fn schema_name() -> String {
            String::schema_name()
        }

        fn is_referenceable() -> bool {
            false
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                extensions: std::iter::once(("contentEncoding", "hex".into()))
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
                ..Default::default()
            }
            .into()
        }
    }
};

/// Contains current valid salt and set of previous
/// salts that can be valid or invalid.
#[cfg_attr(feature = "abi", derive(::borsh::BorshSchema))]
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct SaltRegistry<T: Map<K = Salt, V = bool>> {
    previous: T,
    current: Salt,
}

impl<T: Map<K = Salt, V = bool>> SaltRegistry<T> {
    /// There can be only one valid salt at the beginning
    #[inline]
    pub fn new(map: T, seed: impl Into<[u8; 32]>) -> Self {
        Self {
            previous: map,
            current: Salt::derive(0, seed),
        }
    }

    fn derive_next_salt(&self, seed: impl Into<[u8; 32]>) -> Result<Salt> {
        let seed = seed.into();

        (0..=u8::MAX)
            .map(|num| Salt::derive(num, seed))
            .find(|s| !self.is_used(*s))
            .ok_or(DefuseError::SaltGenerationFailed)
    }

    /// Rotates the current salt, making it previous and keeping it valid.
    #[inline]
    pub fn set_new(&mut self, seed: impl Into<[u8; 32]>) -> Result<Salt> {
        let salt = self.derive_next_salt(seed)?;

        let previous = mem::replace(&mut self.current, salt);
        self.previous.insert(previous, true);

        Ok(previous)
    }

    /// Deactivates the previous salt, making it invalid.
    #[inline]
    pub fn invalidate(&mut self, salt: Salt, seed: impl Into<[u8; 32]>) -> Result<()> {
        if salt == self.current {
            self.set_new(seed)?;
        }

        self.previous
            .get_mut(&salt)
            .map(|v| *v = false)
            .ok_or(DefuseError::InvalidSalt)
    }

    #[inline]
    pub fn is_valid(&self, salt: Salt) -> bool {
        salt == self.current || self.previous.get(&salt).is_some_and(|v| *v)
    }

    #[inline]
    fn is_used(&self, salt: Salt) -> bool {
        salt == self.current || self.previous.contains_key(&salt)
    }

    #[inline]
    pub const fn current(&self) -> Salt {
        self.current
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
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
    use std::collections::BTreeMap;

    use super::*;

    use arbitrary::Unstructured;
    use defuse_test_utils::random::{Rng, RngExt, random_bytes, rng};
    use rstest::rstest;

    impl From<&[u8]> for Salt {
        fn from(value: &[u8]) -> Self {
            let mut result = [0u8; 4];
            result.copy_from_slice(&value[..4]);
            Self(result)
        }
    }

    fn seed_to_salt(seed: &[u8; 32], attempts: u8) -> Salt {
        let seed = [seed, attempts.to_be_bytes().as_ref()].concat();
        let hash = Sha256::digest(&seed);

        hash[..4].into()
    }

    #[rstest]
    fn contains_salt_test(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let random_salt: Salt = Unstructured::new(&random_bytes).arbitrary().unwrap();
        let salts = SaltRegistry::new(BTreeMap::<Salt, bool>::default(), rng.random::<[u8; 32]>());

        assert!(salts.is_valid(salts.current));
        assert!(!salts.is_valid(random_salt));
    }

    #[rstest]
    fn update_current_salt_test(mut rng: impl Rng) {
        let mut salts =
            SaltRegistry::new(BTreeMap::<Salt, bool>::default(), rng.random::<[u8; 32]>());

        let seed = rng.random::<[u8; 32]>();
        let previous_salt = salts.set_new(seed).expect("should set new salt");

        assert!(salts.is_valid(seed_to_salt(&seed, 0)));
        assert!(salts.is_valid(previous_salt));

        let seed = rng.random::<[u8; 32]>();
        let previous_salt = salts.set_new(seed).expect("should set new salt");
        assert!(salts.is_valid(seed_to_salt(&seed, 0)));
        assert!(salts.is_valid(previous_salt));
    }

    #[rstest]
    fn reset_salt_test(mut rng: impl Rng) {
        let mut salts =
            SaltRegistry::new(BTreeMap::<Salt, bool>::default(), rng.random::<[u8; 32]>());
        let random_salt = rng.random::<[u8; 4]>().as_slice().into();

        let seed = rng.random::<[u8; 32]>();
        let current = seed_to_salt(&seed, 0);
        let previous_salt = salts.set_new(seed).expect("should set new salt");

        assert!(
            salts
                .invalidate(previous_salt, rng.random::<[u8; 32]>())
                .is_ok()
        );
        assert!(!salts.is_valid(previous_salt));
        assert!(matches!(
            salts
                .invalidate(random_salt, rng.random::<[u8; 32]>())
                .unwrap_err(),
            DefuseError::InvalidSalt
        ));

        let seed = rng.random::<[u8; 32]>();
        let new_salt = seed_to_salt(&seed, 0);

        assert!(salts.invalidate(current, seed).is_ok());
        assert!(!salts.is_valid(current));
        assert_eq!(salts.current(), new_salt);
    }

    #[rstest]
    fn derive_next_test(mut rng: impl Rng) {
        let mut salt_registry =
            SaltRegistry::new(BTreeMap::<Salt, bool>::default(), rng.random::<[u8; 32]>());

        let prev = salt_registry.set_new(rng.random::<[u8; 32]>()).unwrap();

        salt_registry
            .invalidate(prev, rng.random::<[u8; 32]>())
            .unwrap();
        salt_registry.set_new(rng.random::<[u8; 32]>()).unwrap();

        assert!(!salt_registry.is_valid(prev));
        assert!(salt_registry.is_used(prev));
    }
}
