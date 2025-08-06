//! BIP-322 signature verification modules
//!
//! This module contains address-specific verification logic for different
//! Bitcoin address types. Each module handles the specific requirements
//! for that address format.

pub mod p2pkh;
pub mod p2sh;
pub mod p2wpkh;
pub mod p2wsh;

pub use p2pkh::{compute_p2pkh_message_hash, verify_p2pkh_signature};
pub use p2sh::{compute_p2sh_message_hash, verify_p2sh_signature};
pub use p2wpkh::{compute_p2wpkh_message_hash, verify_p2wpkh_signature};
pub use p2wsh::{compute_p2wsh_message_hash, verify_p2wsh_signature};
