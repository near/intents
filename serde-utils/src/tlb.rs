use std::marker::PhantomData;

use near_sdk::serde::{self, Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};
use tlb_ton::{
    BagOfCellsArgs, BoC, Context,
    r#as::Same,
    de::r#as::CellDeserializeAsOwned,
    ser::{
        CellSerializeExt,
        r#as::{CellSerializeAs, CellSerializeWrapAsExt},
    },
};

pub struct AsBoC<As: ?Sized, CellAs: ?Sized = Same>(PhantomData<As>, PhantomData<CellAs>);

impl<T, As, CellAs> SerializeAs<T> for AsBoC<As, CellAs>
where
    CellAs: CellSerializeAs<T>,
    for<'a> As: SerializeAs<&'a [u8]>,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = source
            .wrap_as::<CellAs>()
            .to_cell()
            .map(BoC::from_root)
            .and_then(|boc| {
                boc.serialize(BagOfCellsArgs {
                    has_idx: false,
                    has_crc32c: false,
                })
                .context("BoC")
            })
            .context("TL-B")
            .map_err(serde::ser::Error::custom)?;

        As::serialize_as(&bytes.as_slice(), serializer)
    }
}

impl<'de, T, As, CellAs> DeserializeAs<'de, T> for AsBoC<As, CellAs>
where
    CellAs: CellDeserializeAsOwned<T>,
    As: DeserializeAs<'de, Vec<u8>>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = As::deserialize_as(deserializer)?;
        BoC::deserialize(bytes)
            .and_then(|boc| boc.into_single_root().context("multiple roots"))
            .context("BoC")
            .and_then(|root| root.parse_fully_as::<T, CellAs>())
            .context("TL-B")
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
const _: () = {
    use near_sdk::schemars::{SchemaGenerator, schema::Schema};
    use serde_with::schemars_0_8::JsonSchemaAs;

    impl<T, As, CellAs> JsonSchemaAs<T> for AsBoC<As, CellAs>
    where
        As: JsonSchemaAs<T>,
    {
        fn schema_name() -> String {
            As::schema_name()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            As::json_schema(generator)
        }

        fn is_referenceable() -> bool {
            As::is_referenceable()
        }
    }
};
