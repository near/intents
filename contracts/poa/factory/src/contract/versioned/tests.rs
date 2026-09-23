use near_sdk::{NearToken, borsh, store::IterableSet};

use super::*;

fn make_legacy_storage() -> ContractStorageV0 {
    let mut tokens = IterableSet::new(b"t".to_vec());
    tokens.insert("usdt".to_string());
    tokens.insert("usdc".to_string());

    ContractStorageV0 {
        tokens,
        bridge_token_storage_deposit_required: NearToken::from_millinear(500),
    }
}

#[test]
fn legacy_upgrade() {
    // legacy state has no versioning wrapper around it
    let legacy = make_legacy_storage();
    let serialized_legacy = borsh::to_vec(&legacy).expect("unable to serialize legacy state");

    // we need to drop it, so all collections flush their pending writes
    drop(legacy);

    let storage: ContractStorage =
        defuse_borsh_utils::As::<MaybeVersionedContractStorage>::deserialize(
            &mut serialized_legacy.as_slice(),
        )
        .expect("failed to deserialize legacy state");

    assert_eq!(storage.tokens.len(), 2);
    assert!(storage.tokens.contains("usdt"));
    assert!(storage.tokens.contains("usdc"));
    assert_eq!(
        storage.bridge_token_storage_deposit_required,
        NearToken::from_millinear(500)
    );
    assert!(!storage.deposits.contains("some-deposit"));
    assert!(storage.withdrawals.get("some-withdrawal").is_none());
    assert!(storage.omni_tokens.is_empty());

    // re-serializing must always produce the versioned (Latest) representation
    let mut serialized_versioned = Vec::new();
    defuse_borsh_utils::As::<MaybeVersionedContractStorage>::serialize(
        &storage,
        &mut serialized_versioned,
    )
    .expect("failed to serialize versioned state");
    assert_ne!(serialized_versioned, serialized_legacy);

    let roundtripped: ContractStorage =
        defuse_borsh_utils::As::<MaybeVersionedContractStorage>::deserialize(
            &mut serialized_versioned.as_slice(),
        )
        .expect("failed to deserialize versioned state");
    assert_eq!(roundtripped.tokens.len(), 2);
    assert!(roundtripped.tokens.contains("usdt"));
    assert!(roundtripped.tokens.contains("usdc"));
}

#[test]
fn versioned_roundtrip() {
    let mut tokens = IterableSet::new(Prefix::Tokens);
    tokens.insert("weth".to_string());

    let mut deposits = LookupSet::new(Prefix::Deposits);
    deposits.insert("deposit-1".to_string());

    let storage = ContractStorage {
        tokens,
        bridge_token_storage_deposit_required: NearToken::from_yoctonear(1),
        deposits,
        withdrawals: LookupMap::new(Prefix::Withdrawals),
        omni_tokens: IterableSet::new(Prefix::OmniTokens),
    };

    let mut serialized = Vec::new();
    defuse_borsh_utils::As::<MaybeVersionedContractStorage>::serialize(&storage, &mut serialized)
        .expect("failed to serialize");
    drop(storage);

    let deserialized: ContractStorage =
        defuse_borsh_utils::As::<MaybeVersionedContractStorage>::deserialize(
            &mut serialized.as_slice(),
        )
        .expect("failed to deserialize");

    assert!(deserialized.tokens.contains("weth"));
    assert!(deserialized.deposits.contains("deposit-1"));
}
