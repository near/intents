use std::collections::{BTreeMap, HashMap};

use rstest::rstest;

use super::*;
use crate::IterableMap;

#[cfg(feature = "near")]
fn near_iterable_map() -> near_sdk::store::IterableMap<String, i32> {
    use near_sdk::{test_utils::VMContextBuilder, testing_env};

    testing_env!(VMContextBuilder::new().build());
    near_sdk::store::IterableMap::new(b"m".to_vec())
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn vacant_entry_dropped_without_mutation_does_not_insert<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    let _ = m.entry_or_default("a".to_string());
    assert!(m.is_empty());
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn vacant_entry_mutated_to_nondefault_inserts_on_drop<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    *m.entry_or_default("a".to_string()) += 1;
    assert_eq!(m.get(&"a".to_string()), Some(&1));
    assert_eq!(m.len(), 1);
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn vacant_entry_mutated_back_to_default_does_not_insert<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    {
        let mut entry = m.entry_or_default("a".to_string());
        *entry += 1;
        *entry -= 1;
    }
    assert!(m.is_empty());
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn occupied_entry_at_default_removed_on_drop<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    m.insert("a".to_string(), 0);
    let _ = m.entry_or_default("a".to_string());
    assert!(m.is_empty());
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn occupied_entry_at_nondefault_kept_untouched_on_drop<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    m.insert("a".to_string(), 5);
    let _ = m.entry_or_default("a".to_string());
    assert_eq!(m.get(&"a".to_string()), Some(&5));
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn occupied_entry_mutated_to_default_removed_on_drop<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    m.insert("a".to_string(), 1);
    *m.entry_or_default("a".to_string()) -= 1;
    assert!(m.is_empty());
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn occupied_entry_mutated_to_new_nondefault_updates_on_drop<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    m.insert("a".to_string(), 1);
    *m.entry_or_default("a".to_string()) += 4;
    assert_eq!(m.get(&"a".to_string()), Some(&5));
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn key_returns_correct_key_for_vacant_entry<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    assert_eq!(m.entry_or_default("a".to_string()).key(), "a");
    // Reading the key alone must not insert anything.
    assert!(m.is_empty());
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn key_returns_correct_key_for_occupied_entry<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    m.insert("a".to_string(), 5);
    assert_eq!(m.entry_or_default("a".to_string()).key(), "a");
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn explicit_remove_on_vacant_returns_default_and_does_not_insert<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    let removed = m.entry_or_default("a".to_string()).remove();
    assert_eq!(removed, 0);
    assert!(m.is_empty());
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn explicit_remove_on_occupied_removes_and_returns_current_value<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    m.insert("a".to_string(), 7);
    let removed = m.entry_or_default("a".to_string()).remove();
    assert_eq!(removed, 7);
    assert!(m.is_empty());
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn deref_and_deref_mut_read_and_write_through_to_the_value<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    let mut entry = m.entry_or_default("a".to_string());
    assert_eq!(*entry, 0);
    *entry = 42;
    assert_eq!(*entry, 42);
    drop(entry);
    assert_eq!(m.get(&"a".to_string()), Some(&42));
}

#[rstest]
#[case::hash_map(HashMap::<String, i32>::new())]
#[case::btree_map(BTreeMap::<String, i32>::new())]
#[cfg_attr(feature = "near", case::near_iterable_map(near_iterable_map()))]
fn multiple_keys_are_independent<M>(#[case] mut m: M)
where
    M: Map<K = String, V = i32> + IterableMap,
{
    *m.entry_or_default("a".to_string()) += 1;
    let _ = m.entry_or_default("b".to_string()); // touched, but left at default -> not inserted
    *m.entry_or_default("c".to_string()) += 3;

    assert_eq!(m.len(), 2);
    assert_eq!(m.get(&"a".to_string()), Some(&1));
    assert_eq!(m.get(&"b".to_string()), None);
    assert_eq!(m.get(&"c".to_string()), Some(&3));
}
