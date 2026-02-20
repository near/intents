use std::collections::BTreeMap;

use near_sdk::borsh::schema::{Declaration, Definition};

use crate::adapters::As;

pub trait BorshSchemaAs<T> {
    fn add_definitions_recursively_as(definitions: &mut BTreeMap<Declaration, Definition>);

    /// Get the name of the type without brackets.
    fn declaration_as() -> Declaration;
}

impl<T: ?Sized> As<T> {
    pub fn add_definitions_recursively<U>(definitions: &mut BTreeMap<Declaration, Definition>)
    where
        T: BorshSchemaAs<U>,
    {
        T::add_definitions_recursively_as(definitions)
    }

    pub fn declaration<U>() -> Declaration
    where
        T: BorshSchemaAs<U>,
    {
        T::declaration_as()
    }
}
