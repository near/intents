use std::collections::HashMap;

use super::*;

#[test]
fn vacant_entry_dropped_without_mutation_does_not_insert() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    let _ = m.entry_or_default("a");
    assert!(m.is_empty());
}

#[test]
fn vacant_entry_mutated_to_nondefault_inserts_on_drop() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    *m.entry_or_default("a") += 1;
    assert_eq!(m.get("a"), Some(&1));
    assert_eq!(m.len(), 1);
}

#[test]
fn vacant_entry_mutated_back_to_default_does_not_insert() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    {
        let mut entry = m.entry_or_default("a");
        *entry += 1;
        *entry -= 1;
    }
    assert!(m.is_empty());
}

#[test]
fn occupied_entry_at_default_removed_on_drop() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    m.insert("a", 0);
    let _ = m.entry_or_default("a");
    assert!(m.is_empty());
}

#[test]
fn occupied_entry_at_nondefault_kept_untouched_on_drop() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    m.insert("a", 5);
    let _ = m.entry_or_default("a");
    assert_eq!(m.get("a"), Some(&5));
}

#[test]
fn occupied_entry_mutated_to_default_removed_on_drop() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    m.insert("a", 1);
    *m.entry_or_default("a") -= 1;
    assert!(m.is_empty());
}

#[test]
fn occupied_entry_mutated_to_new_nondefault_updates_on_drop() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    m.insert("a", 1);
    *m.entry_or_default("a") += 4;
    assert_eq!(m.get("a"), Some(&5));
}

#[test]
fn key_returns_correct_key_for_vacant_entry() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    assert_eq!(*m.entry_or_default("a").key(), "a");
    // Reading the key alone must not insert anything.
    assert!(m.is_empty());
}

#[test]
fn key_returns_correct_key_for_occupied_entry() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    m.insert("a", 5);
    assert_eq!(*m.entry_or_default("a").key(), "a");
}

#[test]
fn explicit_remove_on_vacant_returns_default_and_does_not_insert() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    let removed = m.entry_or_default("a").remove();
    assert_eq!(removed, 0);
    assert!(m.is_empty());
}

#[test]
fn explicit_remove_on_occupied_removes_and_returns_current_value() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    m.insert("a", 7);
    let removed = m.entry_or_default("a").remove();
    assert_eq!(removed, 7);
    assert!(m.is_empty());
}

#[test]
fn deref_and_deref_mut_read_and_write_through_to_the_value() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    let mut entry = m.entry_or_default("a");
    assert_eq!(*entry, 0);
    *entry = 42;
    assert_eq!(*entry, 42);
    drop(entry);
    assert_eq!(m.get("a"), Some(&42));
}

#[test]
fn multiple_keys_are_independent() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    *m.entry_or_default("a") += 1;
    let _ = m.entry_or_default("b"); // touched, but left at default -> not inserted
    *m.entry_or_default("c") += 3;

    assert_eq!(m.len(), 2);
    assert_eq!(m.get("a"), Some(&1));
    assert_eq!(m.get("b"), None);
    assert_eq!(m.get("c"), Some(&3));
}
