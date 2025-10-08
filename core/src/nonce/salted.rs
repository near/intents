use core::mem;
use hex::FromHex;
use impl_tools::autoimpl;
use near_sdk::{
    IntoStorageKey,
    borsh::{BorshDeserialize, BorshSerialize},
    env, near,
    store::{IterableMap, key::Identity},
};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{
    fmt::{self, Debug},
    str::FromStr,
};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(PartialEq, PartialOrd, Ord, Eq, Copy, Clone, SerializeDisplay, DeserializeFromStr)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[autoimpl(AsRef using self.0)]
#[autoimpl(AsMut using self.0)]
#[near(serializers = [borsh])]
pub struct Salt([u8; 4]);

impl Salt {
    pub fn random() -> Self {
        const SIZE: usize = size_of::<Salt>();
        let seed = &env::random_seed_array()[..SIZE];
        let mut result = [0u8; SIZE];

        result.copy_from_slice(seed);

        Self(result)
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

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi {
    use super::*;

    use near_sdk::{
        schemars::{
            JsonSchema,
            r#gen::SchemaGenerator,
            schema::{InstanceType, Metadata, Schema, SchemaObject},
        },
        serde_json,
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
                extensions: [("contentEncoding", "hex".into())]
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
                ..Default::default()
            }
            .into()
        }
    }
}

/// Contains current valid salt and set of previous
/// salts that can be valid or invalid.
#[near(serializers = [borsh])]
#[derive(Debug)]
pub struct SaltRegistry {
    previous: IterableMap<Salt, bool, Identity>,
    current: Salt,
}

impl SaltRegistry {
    /// There can be only one valid salt at the beginning
    #[inline]
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self {
            previous: IterableMap::with_hasher(prefix),
            current: Salt::random(),
        }
    }

    /// Rotates the current salt, making the previous salt valid as well.
    #[inline]
    pub fn set_new(&mut self) -> Salt {
        let salt = Salt::random();
        let previous = mem::replace(&mut self.current, salt);

        self.previous.insert(previous, true);

        previous
    }

    /// Deactivates the previous salt, making it invalid.
    #[inline]
    pub fn invalidate(&mut self, salt: Salt) -> bool {
        if salt == self.current {
            self.set_new();
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

    impl From<&[u8]> for Salt {
        fn from(value: &[u8]) -> Self {
            let mut result = [0u8; 4];
            result.copy_from_slice(&value[..4]);
            Self(result)
        }
    }

    fn set_random_seed(rng: &mut impl Rng) -> Salt {
        let seed = rng.random();
        let context = VMContextBuilder::new().random_seed(seed).build();
        testing_env!(context);

        seed[..4].into()
    }

    #[rstest]
    fn contains_salt_test(random_bytes: Vec<u8>) {
        let random_salt: Salt = Unstructured::new(&random_bytes).arbitrary().unwrap();
        let salts = SaltRegistry::new(random_bytes);

        assert!(salts.is_valid(salts.current));
        assert!(!salts.is_valid(random_salt));
    }

    #[rstest]
    fn update_current_salt_test(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let mut salts = SaltRegistry::new(random_bytes);

        let current = set_random_seed(&mut rng);
        let previous = salts.set_new();

        assert!(salts.is_valid(current));
        assert!(salts.is_valid(previous));
    }

    #[rstest]
    fn reset_salt_test(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let mut salts = SaltRegistry::new(random_bytes);
        let random_salt = rng.random::<[u8; 4]>().as_slice().into();

        let current = set_random_seed(&mut rng);
        let previous = salts.set_new();

        assert!(salts.invalidate(previous));
        assert!(!salts.is_valid(previous));
        assert!(!salts.invalidate(random_salt));

        let new_salt = set_random_seed(&mut rng);
        assert!(salts.invalidate(current));
        assert!(!salts.is_valid(current));
        assert_eq!(salts.current(), new_salt);
    }
}
