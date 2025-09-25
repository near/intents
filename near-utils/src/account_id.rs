use near_account_id::AccountType;
use near_sdk::AccountIdRef;

pub fn is_derived_from_public_key(account_id: &AccountIdRef) -> bool {
    match account_id.get_account_type() {
        AccountType::NearImplicitAccount => true,
        AccountType::EthImplicitAccount => true,
        AccountType::NamedAccount => false,
    }
}
