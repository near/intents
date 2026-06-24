use core::{ops::Add, time::Duration};

pub trait Now: Sized {
    #[must_use]
    fn now() -> Self;

    #[must_use]
    #[inline]
    fn timeout(timeout: Duration) -> Self
    where
        Self: Add<Duration, Output = Self>,
    {
        Self::now() + timeout
    }

    #[inline]
    fn has_passed(&self) -> bool
    where
        Self: PartialOrd,
    {
        *self < Self::now()
    }
}

#[cfg(feature = "jiff")]
const _: () = {
    use jiff::Timestamp;

    impl Now for Timestamp {
        #[track_caller]
        #[inline]
        fn now() -> Self {
            cfg_select! {
                near => {
                    Self::from_nanosecond(
                        ::near_sdk::env::block_timestamp().into(),
                    ).expect("UNIX timestamp: out of range")
                }
                _ => Self::now(),
            }
        }
    }
};

// #[cfg(feature = "borsh")]
// mod borsh;
// #[cfg(feature = "serde")]
// mod serde;

// use core::{
//     ops::{Add, AddAssign, Sub, SubAssign},
//     time::Duration,
// };
// use time::{OffsetDateTime, UtcDateTime};

// #[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
// #[cfg_attr(
//     feature = "serde",
//     ::cfg_eval::cfg_eval,
//     ::serde_with::serde_as,
//     derive(::serde::Serialize, ::serde::Deserialize),
//     cfg_attr(
//         feature = "abi",
//         derive(::schemars::JsonSchema),
//         schemars(example = "Self::example")
//     )
// )]
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// #[repr(transparent)]
// pub struct DateTime(
//     // TODO: serde
//     // #[cfg_attr(feature = "serde", serde(with = "::time::serde::rfc3339"))]
//     #[cfg_attr(all(feature = "serde", feature = "abi"), schemars(with = "String"))] UtcDateTime,
// );

// impl DateTime {
//     pub const UNIX_EPOCH: Self = Self(::time::UtcDateTime::UNIX_EPOCH);
//     // TODO: large-dates
//     pub const MAX: Self = Self(::time::UtcDateTime::MAX);

//     #[cfg(feature = "std")]
//     #[must_use]
//     #[inline]
//     pub fn now() -> Self {
//         Self(cfg_select! {
//             near => {
//                 UtcDateTime::from_unix_timestamp_nanos(
//                     ::near_sdk::env::block_timestamp().into(),
//                 ).expect("UNIX timestamp: out of range")
//             }
//             _ => UtcDateTime::now(),
//         })
//     }

//     #[cfg(feature = "std")]
//     #[must_use]
//     #[inline]
//     pub fn timeout(timeout: Duration) -> Self {
//         Self::now() + timeout
//     }

//     #[cfg(feature = "std")]
//     #[must_use]
//     #[inline]
//     pub fn has_passed(&self) -> bool {
//         *self < Self::now()
//     }

//     /// Truncate `Deadline` down to seconds part.
//     /// E.g. `2026-03-10T09:32:16.123Z` would be truncated down to
//     /// `2026-03-10T09:32:16Z`
//     #[must_use]
//     #[inline]
//     pub fn truncate_to_second(self) -> Self {
//         Self(self.into_inner().truncate_to_second())
//     }

//     #[must_use]
//     #[inline]
//     pub const fn into_inner(self) -> UtcDateTime {
//         self.0
//     }

//     #[cfg(all(feature = "serde", feature = "abi"))]
//     fn example() -> Self {
//         use time::{Date, Month, Time};

//         UtcDateTime::new(
//             Date::from_calendar_date(2026, Month::June, 21).unwrap(),
//             Time::from_hms(13, 45, 57).unwrap(),
//         )
//         .into()
//     }
// }

// impl From<UtcDateTime> for DateTime {
//     fn from(value: UtcDateTime) -> Self {
//         Self(value)
//     }
// }

// impl From<OffsetDateTime> for DateTime {
//     fn from(value: OffsetDateTime) -> Self {
//         Self(value.to_utc())
//     }
// }

// impl From<DateTime> for UtcDateTime {
//     fn from(value: DateTime) -> Self {
//         value.into_inner()
//     }
// }

// impl Add<Duration> for DateTime {
//     type Output = Self;

//     #[inline]
//     fn add(self, rhs: Duration) -> Self::Output {
//         Self(self.0 + rhs)
//     }
// }

// impl Add<::time::Duration> for DateTime {
//     type Output = Self;

//     fn add(self, rhs: ::time::Duration) -> Self::Output {
//         Self(self.0 + rhs)
//     }
// }

// impl Sub<Duration> for DateTime {
//     type Output = Self;

//     fn sub(self, rhs: Duration) -> Self::Output {
//         Self(self.0 - rhs)
//     }
// }

// impl Sub<::time::Duration> for DateTime {
//     type Output = Self;

//     fn sub(self, rhs: ::time::Duration) -> Self::Output {
//         Self(self.0 - rhs)
//     }
// }

// impl AddAssign<Duration> for DateTime {
//     #[inline]
//     fn add_assign(&mut self, rhs: Duration) {
//         self.0 += rhs;
//     }
// }

// impl AddAssign<::time::Duration> for DateTime {
//     #[inline]
//     fn add_assign(&mut self, rhs: ::time::Duration) {
//         self.0 += rhs;
//     }
// }

// impl SubAssign<Duration> for DateTime {
//     fn sub_assign(&mut self, rhs: Duration) {
//         self.0 -= rhs;
//     }
// }

// impl SubAssign<::time::Duration> for DateTime {
//     fn sub_assign(&mut self, rhs: ::time::Duration) {
//         self.0 -= rhs;
//     }
// }

// // #[cfg(feature = "std")]
// // pub fn now() -> UtcDateTime {
// //     cfg_select! {
// //         // near => {
// //         //     Self::from_timestamp_nanos(
// //         //         ::near_sdk::env::block_timestamp()
// //         //             .try_into()
// //         //             .expect("out of range")
// //         //     )
// //         // }
// //         _ => UtcDateTime::now(),
// //     }
// // }

// // use core::ops::Add;

// // pub use chrono::{self, *};

// // pub type DateTime<Tz = Utc> = ::chrono::DateTime<Tz>;

// // pub trait Now: Sized {
// //     fn now() -> Self;

// //     #[inline]
// //     fn timeout(timeout: Duration) -> Self
// //     where
// //         Self: Add<::core::time::Duration, Output = Self>,
// //     {
// //         Self::now() + timeout
// //     }

// //     #[inline]
// //     fn timeout_std(timeout: ::core::time::Duration) -> Self
// //     where
// //         Self: Add<::core::time::Duration, Output = Self>,
// //     {
// //         Self::now() + timeout
// //     }

// //     #[inline]
// //     fn has_passed(&self) -> bool
// //     where
// //         Self: PartialOrd,
// //     {
// //         *self < Self::now()
// //     }
// // }

// // impl Now for DateTime<Utc> {
// //     #[inline]
// //     fn now() -> Self {
// //         cfg_select! {
// //             near => {
// //                 Self::from_timestamp_nanos(
// //                     ::near_sdk::env::block_timestamp()
// //                         .try_into()
// //                         .expect("out of range")
// //                 )
// //             }
// //             _ => Utc::now(),
// //         }
// //     }
// // }

// // // #[inline]
// // // pub fn now() -> DateTime<Utc> {
// // //     cfg_select! {
// // //         near => {
// // //             ::chrono::DateTime::from_timestamp_nanos(
// // //                 ::near_sdk::env::block_timestamp()
// // //                     .try_into()
// // //                     .expect("out of range")
// // //             )
// // //         }
// // //         _ => Utc::now(),
// // //     }
// // // }

// // // #[inline]
// // // pub fn timeout(timeout: Duration) -> DateTime<Utc> {
// // //     now() + timeout
// // // }
