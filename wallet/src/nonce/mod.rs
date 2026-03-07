mod seqno;

pub use self::seqno::*;

#[cfg(feature = "highload")]
mod highload;
#[cfg(feature = "highload")]
pub use self::highload::*;

use core::fmt::Display;

pub trait Nonces {
    type Nonce;
    type Error: Display;

    fn commit(&mut self, nonce: Self::Nonce) -> Result<(), Self::Error>;
}
