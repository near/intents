use defuse_outlayer_sdk as sdk;

fn main() {
    let derivation_path = "path";

    let public_key = sdk::host::crypto::secp256k1::derive_public_key(derivation_path);

    let prehash = [42u8; 32];
    let signature = sdk::host::crypto::secp256k1::sign(derivation_path, &prehash);
}

#[cfg(test)]
mod tests {
    use defuse_outlayer_sdk::AccountId;

    use super::*;

    #[test]
    fn test() {
        let path = "path";
        let msg = "message";

        let pk = sdk::host::crypto::ed25519::derive_public_key(path);
        println!("pk: {pk:?}");

        let sig = sdk::host::crypto::ed25519::sign(path, msg);
        println!("sig: {sig:?}");
    }
}
