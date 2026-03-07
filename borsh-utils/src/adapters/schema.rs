use std::collections::BTreeMap;

use near_sdk::borsh::schema::{Declaration, Definition};

use super::As;

pub trait BorshSchemaAs<T: ?Sized> {
    fn declaration_as() -> Declaration;
    fn add_definitions_recursively_as(definitions: &mut BTreeMap<Declaration, Definition>);
}

impl<T: ?Sized> As<T> {
    pub fn declaration<U: ?Sized>() -> Declaration
    where
        T: BorshSchemaAs<U>,
    {
        T::declaration_as()
    }

    pub fn add_definitions_recursively<U: ?Sized>(
        definitions: &mut BTreeMap<Declaration, Definition>,
    ) where
        T: BorshSchemaAs<U>,
    {
        T::add_definitions_recursively_as(definitions);
    }
}
