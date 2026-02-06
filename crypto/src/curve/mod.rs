#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "ed25519")]
pub use self::ed25519::*;

#[cfg(feature = "secp256k1")]
mod secp256k1;
#[cfg(feature = "secp256k1")]
pub use self::secp256k1::*;

#[cfg(feature = "p256")]
mod p256;
#[cfg(feature = "p256")]
pub use self::p256::*;

use crate::{ParseCurveError, parse::checked_base58_decode_array};

use near_sdk::bs58;
use strum::{Display, EnumString, IntoStaticStr};

pub trait Curve {
    type PublicKey;
    type Signature;

    /// Message that can be signed by this curve
    type Message: AsRef<[u8]> + ?Sized;

    /// Public key that should be known prior to verification
    type VerifyingKey;

    fn verify(
        signature: &Self::Signature,
        message: &Self::Message,
        verifying_key: &Self::VerifyingKey,
    ) -> Option<Self::PublicKey>;
}

#[derive(Display, IntoStaticStr, EnumString)]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
#[repr(u8)]
pub enum CurveType {
    #[cfg(feature = "ed25519")]
    Ed25519 = 0,
    #[cfg(feature = "secp256k1")]
    Secp256k1 = 1,
    #[cfg(feature = "p256")]
    P256 = 2,
}

pub trait TypedCurve: Curve {
    const CURVE_TYPE: CurveType;

    #[inline]
    fn to_base58(bytes: impl AsRef<[u8]>) -> String {
        format!(
            "{}:{}",
            Self::CURVE_TYPE,
            bs58::encode(bytes.as_ref()).into_string()
        )
    }

    fn parse_base58<const N: usize>(s: impl AsRef<str>) -> Result<[u8; N], ParseCurveError> {
        let s = s.as_ref();
        let data = if let Some((curve, data)) = s.split_once(':') {
            if !curve.eq_ignore_ascii_case(Self::CURVE_TYPE.into()) {
                return Err(ParseCurveError::WrongCurveType);
            }
            data
        } else {
            s
        };
        checked_base58_decode_array(data)
    }
}
