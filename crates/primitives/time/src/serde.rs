use serde_with::{
    DeserializeAs, SerializeAs, TimestampMicroSeconds, TimestampMicroSecondsWithFrac,
    TimestampMilliSeconds, TimestampMilliSecondsWithFrac, TimestampNanoSeconds,
    TimestampNanoSecondsWithFrac, TimestampSeconds, TimestampSecondsWithFrac, formats,
};

macro_rules! impl_serde_as {
        ($($a:ident,)+) => {$(
            impl<FORMAT: formats::Format, STRICTNESS: formats::Strictness> SerializeAs<DateTime>
                for $a<FORMAT, STRICTNESS>
            where
                $a<FORMAT, STRICTNESS>: SerializeAs<::chrono::DateTime<Utc>>,
            {
                fn serialize_as<S>(source: &DateTime, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    Self::serialize_as(&source.0, serializer)
                }
            }

            impl<'de, FORMAT: formats::Format, STRICTNESS: formats::Strictness> DeserializeAs<'de, DateTime>
                for $a<FORMAT, STRICTNESS>
            where
                $a<FORMAT, STRICTNESS>: DeserializeAs<'de, ::chrono::DateTime<Utc>>,
            {
                fn deserialize_as<D>(deserializer: D) -> Result<DateTime, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    Self::deserialize_as(deserializer).map(DateTime)
                }
            }
        )+};
    }

impl_serde_as! {
    TimestampSeconds, TimestampSecondsWithFrac,
    TimestampMilliSeconds, TimestampMilliSecondsWithFrac,
    TimestampMicroSeconds, TimestampMicroSecondsWithFrac,
    TimestampNanoSeconds, TimestampNanoSecondsWithFrac,
}
