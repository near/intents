use std::str::FromStr;

use near_contract_standards::fungible_token::Balance;
use near_sdk::test_utils::{VMContextBuilder, accounts};
use near_sdk::{Gas, testing_env};

use super::*;

const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

fn current() -> AccountId {
    accounts(0)
}

fn owner() -> AccountId {
    accounts(1)
}

fn user1() -> AccountId {
    accounts(2)
}

fn user2() -> AccountId {
    accounts(3)
}

fn setup() -> (Contract, VMContextBuilder) {
    let mut context = VMContextBuilder::new();

    let contract = Contract::new_default_meta(
        owner(),
        TOTAL_SUPPLY.into(),
        "wwNEAR",
        AccountId::from_str("wwrap.near").unwrap(),
    );

    context.storage_usage(env::storage_usage());
    context.current_account_id(current());

    testing_env!(context.build());

    (contract, context)
}

#[test]
fn test_new() {
    let (contract, _) = setup();

    assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
    assert_eq!(contract.ft_balance_of(owner()).0, TOTAL_SUPPLY);
}

#[test]
fn test_metadata() {
    let (contract, _) = setup();

    assert_eq!(contract.ft_metadata().decimals, 24);
    assert!(contract.ft_metadata().icon.is_none());
    assert!(!contract.ft_metadata().spec.is_empty());
    assert!(!contract.ft_metadata().name.is_empty());
    assert!(!contract.ft_metadata().symbol.is_empty());
}

#[test]
#[should_panic(expected = "The contract is not initialized")]
fn test_default_panics() {
    Contract::default();
}

#[test]
fn test_deposit() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    assert!(contract.storage_balance_of(user1()).is_none());

    contract.storage_deposit(None, None);

    let storage_balance = contract.storage_balance_of(user1()).unwrap();
    assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
    assert!(storage_balance.available.is_zero());
}

#[test]
fn test_deposit_on_behalf_of_another_user() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    assert!(contract.storage_balance_of(user2()).is_none());

    // predecessor is user1, but deposit is for user2
    contract.storage_deposit(Some(user2()), None);

    let storage_balance = contract.storage_balance_of(user2()).unwrap();
    assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
    assert!(storage_balance.available.is_zero());

    // ensure that user1's storage wasn't affected
    assert!(contract.storage_balance_of(user1()).is_none());
}

#[should_panic(expected = "The attached deposit is less than the minimum storage balance")]
#[test]
fn test_deposit_panics_on_less_amount() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(100))
            .build()
    );

    assert!(contract.storage_balance_of(user1()).is_none());

    // this panics
    contract.storage_deposit(None, None);
}

#[test]
fn test_deposit_account_twice() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    // this registers the predecessor
    contract.storage_deposit(None, None);

    let storage_balance = contract.storage_balance_of(user1()).unwrap();
    assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);

    // this doesn't panic, and just refunds the deposit as the account is registered already
    contract.storage_deposit(None, None);

    // this indicates that total balance hasn't changed
    let storage_balance = contract.storage_balance_of(user1()).unwrap();
    assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
}

#[test]
fn test_unregister() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    contract.storage_deposit(None, None);

    assert!(contract.storage_balance_of(user1()).is_some());

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );

    assert!(contract.storage_unregister(None));

    assert!(contract.storage_balance_of(user1()).is_none());
}

#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
#[test]
fn test_unregister_panics_on_zero_deposit() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    contract.storage_deposit(None, None);

    assert!(contract.storage_balance_of(user1()).is_some());

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(0))
            .build()
    );

    contract.storage_unregister(None);
}

#[test]
fn test_unregister_of_non_registered_account() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );

    // "false" indicates that the account wasn't registered
    assert!(!contract.storage_unregister(None));
}

// #[should_panic(expected)]
// #[test]
// fn test_unregister_panics_on_non_zero_balance() {
//     let (mut contract, mut context) = setup();

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(contract.storage_balance_bounds().min)
//             .build()
//     );

//     contract.storage_deposit(None, None);

//     assert!(contract.storage_balance_of(user1()).is_some());

//     testing_env!(
//         context
//             .predecessor_account_id(owner())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );
//     let transfer_amount = TOTAL_SUPPLY / 10;

//     contract.ft_transfer(user1(), transfer_amount.into(), None);

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );

//     contract.storage_unregister(None);
// }

// FIXME: this test
// #[test]
// fn test_unregister_with_force() {
//     let (mut contract, mut context) = setup();

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(contract.storage_balance_bounds().min)
//             .build()
//     );

//     contract.storage_deposit(None, None);

//     assert!(contract.storage_balance_of(user1()).is_some());

//     testing_env!(
//         context
//             .predecessor_account_id(owner())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );
//     let transfer_amount = TOTAL_SUPPLY / 10;

//     contract.ft_transfer(user1(), transfer_amount.into(), None);

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );

//     // force to unregister no matter what
//     // this reduces total supply because user's tokens are burnt
//     assert_eq!(contract.storage_unregister(Some(true)), true);

//     assert!(contract.storage_balance_of(user1()).is_none());
//     assert_eq!(contract.ft_balance_of(user1()).0, 0);
//     assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY - transfer_amount);
// }

#[test]
fn test_withdraw() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    contract.storage_deposit(None, None);

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );

    // Basic Fungible Token implementation never transfers Near to caller
    // See: https://github.com/near/near-sdk-rs/blob/5a4c595125364ffe8d7866aa0418a3c92b1c3a6a/near-contract-standards/src/fungible_token/storage_impl.rs#L82
    let storage_balance = contract.storage_withdraw(None);
    assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
    assert!(storage_balance.available.is_zero());

    // Basic Fungible Token implementation never transfers Near to caller
    // See: https://github.com/near/near-sdk-rs/blob/5a4c595125364ffe8d7866aa0418a3c92b1c3a6a/near-contract-standards/src/fungible_token/storage_impl.rs#L82
    let storage_balance = contract.storage_withdraw(None);
    assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
    assert!(storage_balance.available.is_zero());
}

#[should_panic(expected = "The account charlie is not registered")]
#[test]
fn test_withdraw_panics_on_non_registered_account() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );

    contract.storage_withdraw(None);
}

#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
#[test]
fn test_withdraw_panics_on_zero_deposit() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(0))
            .build()
    );

    contract.storage_withdraw(None);
}

#[should_panic(expected = "The account charlie is not registered")]
#[test]
fn test_withdraw_panics_on_amount_greater_than_zero() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );

    // Basic Fungible Token implementation sets storage_balance_bounds.min == storage_balance_bounds.max
    // which means available balance will always be 0
    // See: https://github.com/near/near-sdk-rs/blob/5a4c595125364ffe8d7866aa0418a3c92b1c3a6a/near-contract-standards/src/fungible_token/storage_impl.rs#L82
    contract.storage_withdraw(Some(NearToken::from_yoctonear(1)));
}

// FIXME: All commented tests

// #[test]
// fn test_transfer() {
//     let (mut contract, mut context) = setup();

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(contract.storage_balance_bounds().min)
//             .build()
//     );

//     // Paying for account registration of user1, aka storage deposit
//     contract.storage_deposit(None, None);

//     testing_env!(
//         context
//             .predecessor_account_id(owner())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );
//     let transfer_amount = TOTAL_SUPPLY / 10;

//     contract.ft_transfer(user1(), transfer_amount.into(), None);

//     assert_eq!(
//         contract.ft_balance_of(owner()).0,
//         (TOTAL_SUPPLY - transfer_amount)
//     );
//     assert_eq!(contract.ft_balance_of(user1()).0, transfer_amount);
// }

// #[should_panic]
// #[test]
// fn test_transfer_panics_on_self_receiver() {
//     let (mut contract, mut context) = setup();

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(contract.storage_balance_bounds().min)
//             .build()
//     );

//     // Paying for account registration of user1, aka storage deposit
//     contract.storage_deposit(None, None);

//     testing_env!(
//         context
//             .predecessor_account_id(owner())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );
//     let transfer_amount = TOTAL_SUPPLY / 10;

//     contract.ft_transfer(owner(), transfer_amount.into(), None);
// }

// #[should_panic]
// #[test]
// fn test_transfer_panics_on_zero_amount() {
//     let (mut contract, mut context) = setup();

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(contract.storage_balance_bounds().min)
//             .build()
//     );

//     // Paying for account registration of user1, aka storage deposit
//     contract.storage_deposit(None, None);

//     testing_env!(
//         context
//             .predecessor_account_id(owner())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );

//     contract.ft_transfer(user1(), 0.into(), None);
// }

// #[should_panic]
// #[test]
// fn test_transfer_panics_on_zero_deposit() {
//     let (mut contract, mut context) = setup();

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(contract.storage_balance_bounds().min)
//             .build()
//     );

//     // Paying for account registration of user1, aka storage deposit
//     contract.storage_deposit(None, None);

//     testing_env!(
//         context
//             .predecessor_account_id(owner())
//             .attached_deposit(NearToken::from_yoctonear(0))
//             .build()
//     );

//     let transfer_amount = TOTAL_SUPPLY / 10;
//     contract.ft_transfer(user1(), transfer_amount.into(), None);
// }

// #[should_panic(expected)]
// #[test]
// fn test_transfer_panics_on_non_registered_sender() {
//     let (mut contract, mut context) = setup();

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );

//     let transfer_amount = TOTAL_SUPPLY / 10;
//     contract.ft_transfer(user1(), transfer_amount.into(), None);
// }

// #[should_panic]
// #[test]
// fn test_transfer_panics_on_non_registered_receiver() {
//     let (mut contract, mut context) = setup();

//     testing_env!(
//         context
//             .predecessor_account_id(owner())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );

//     let transfer_amount = TOTAL_SUPPLY / 10;
//     contract.ft_transfer(user1(), transfer_amount.into(), None);
// }

// #[should_panic]
// #[test]
// fn test_transfer_panics_on_amount_greater_than_balance() {
//     let (mut contract, mut context) = setup();

//     testing_env!(
//         context
//             .predecessor_account_id(user1())
//             .attached_deposit(contract.storage_balance_bounds().min)
//             .build()
//     );

//     // Paying for account registration of user1, aka storage deposit
//     contract.storage_deposit(None, None);

//     testing_env!(
//         context
//             .predecessor_account_id(owner())
//             .attached_deposit(NearToken::from_yoctonear(1))
//             .build()
//     );

//     let transfer_amount = TOTAL_SUPPLY + 10;
//     contract.ft_transfer(user1(), transfer_amount.into(), None);
// }

#[test]
fn test_transfer_call() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    // Paying for account registration of user1, aka storage deposit
    contract.storage_deposit(None, None);

    testing_env!(
        context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );
    let transfer_amount = TOTAL_SUPPLY / 10;

    contract.ft_transfer_call(user1(), transfer_amount.into(), None, String::new());

    assert_eq!(
        contract.ft_balance_of(owner()).0,
        (TOTAL_SUPPLY - transfer_amount)
    );
    assert_eq!(contract.ft_balance_of(user1()).0, transfer_amount);
}

#[should_panic(expected = "Sender and receiver should be different")]
#[test]
fn test_transfer_call_panics_on_self_receiver() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    // Paying for account registration of user1, aka storage deposit
    contract.storage_deposit(None, None);

    testing_env!(
        context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );
    let transfer_amount = TOTAL_SUPPLY / 10;

    contract.ft_transfer_call(owner(), transfer_amount.into(), None, String::new());
}

#[should_panic(expected = "The amount should be a positive number")]
#[test]
fn test_transfer_call_panics_on_zero_amount() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    // Paying for account registration of user1, aka storage deposit
    contract.storage_deposit(None, None);

    testing_env!(
        context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );

    contract.ft_transfer_call(user1(), 0.into(), None, String::new());
}

#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
#[test]
fn test_transfer_call_panics_on_zero_deposit() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    // Paying for account registration of user1, aka storage deposit
    contract.storage_deposit(None, None);

    testing_env!(
        context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(0))
            .build()
    );

    let transfer_amount = TOTAL_SUPPLY / 10;
    contract.ft_transfer_call(user1(), transfer_amount.into(), None, String::new());
}

#[should_panic(expected = "Sender and receiver should be different")]
#[test]
fn test_transfer_call_panics_on_non_registered_sender() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );

    let transfer_amount = TOTAL_SUPPLY / 10;
    contract.ft_transfer_call(user1(), transfer_amount.into(), None, String::new());
}

#[should_panic(expected = "The account charlie is not registered")]
#[test]
fn test_transfer_call_panics_on_non_registered_receiver() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );

    let transfer_amount = TOTAL_SUPPLY / 10;
    contract.ft_transfer_call(user1(), transfer_amount.into(), None, String::new());
}

#[should_panic(expected = "The account doesn't have enough balance")]
#[test]
fn test_transfer_call_panics_on_amount_greater_than_balance() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    // Paying for account registration of user1, aka storage deposit
    contract.storage_deposit(None, None);

    testing_env!(
        context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build()
    );

    let transfer_amount = TOTAL_SUPPLY + 10;
    contract.ft_transfer_call(user1(), transfer_amount.into(), None, String::new());
}

#[should_panic(expected = "More gas is required")]
#[test]
fn test_transfer_call_panics_on_unsufficient_gas() {
    let (mut contract, mut context) = setup();

    testing_env!(
        context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build()
    );

    // Paying for account registration of user1, aka storage deposit
    contract.storage_deposit(None, None);

    testing_env!(
        context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .prepaid_gas(Gas::from_tgas(10))
            .build()
    );
    let transfer_amount = TOTAL_SUPPLY / 10;

    contract.ft_transfer_call(user1(), transfer_amount.into(), None, String::new());
}
