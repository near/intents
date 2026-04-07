#![no_std]

unsafe extern "C" {
    #[cfg(feature = "secp256k1")]
    pub fn secp256k1_get_root_public_key(out_ptr: u64);
    #[cfg(feature = "secp256k1")]
    pub fn secp256k1_derive_public_key(path_ptr: u64, path_len: u64, out_ptr: u64);
    #[cfg(feature = "secp256k1")]
    pub fn secp256k1_sign(path_ptr: u64, path_len: u64, msg_ptr: u64, msg_len: u64, out_ptr: u64);

    #[cfg(feature = "ed25519")]
    pub fn ed25519_get_root_public_key(out_ptr: u64);
    #[cfg(feature = "ed25519")]
    pub fn ed25519_derive_public_key(path_ptr: u64, path_len: u64, out_ptr: u64);
    #[cfg(feature = "ed25519")]
    pub fn ed25519_sign(path_ptr: u64, path_len: u64, msg_ptr: u64, msg_len: u64, out_ptr: u64);
}
