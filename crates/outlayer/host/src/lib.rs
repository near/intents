pub mod crypto;

use self::crypto::CryptoHost;

pub trait Host: CryptoHost {}
impl<T> Host for T where T: CryptoHost {}
