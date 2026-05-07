use super::As;
use borsh::schema::{Declaration, Definition};
use std::collections::BTreeMap;

pub trait BorshSchemaAs<T: ?Sized> {
    fn declaration_as() -> Declaration;
    fn add_definitions_recursively_as(definitions: &mut BTreeMap<Declaration, Definition>);
}

macro_rules! impl_borsh_schema_as {
    ($target:ty, $adapter:ident) => {
        impl<I> $crate::adapters::BorshSchemaAs<$target> for $adapter<I>
        where
            I: borsh::BorshSchema,
        {
            fn declaration_as() -> borsh::schema::Declaration {
                <I as borsh::BorshSchema>::declaration()
            }

            fn add_definitions_recursively_as(
                definitions: &mut std::collections::BTreeMap<
                    borsh::schema::Declaration,
                    borsh::schema::Definition,
                >,
            ) {
                <I as borsh::BorshSchema>::add_definitions_recursively(definitions);
            }
        }
    };
}
pub(crate) use impl_borsh_schema_as;

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
