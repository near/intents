use defuse_outlayer_sdk as sdk;

fn main() {
    let derivation_path = "path";

    let public_key = sdk::host::crypto::ed25519::derive_public_key(derivation_path);

    let msg = "message";
    let signature = sdk::host::crypto::ed25519::sign(derivation_path, msg);
}

#[cfg(test)]
mod tests {
    use defuse_outlayer_sdk::AccountId;

    use super::*;

    #[test]
    fn test() {
        let path = "path";

        let pk = sdk::host::crypto::ed25519::derive_public_key(path);
        println!("{pk:?}");

        sdk::host::mock::with(|h| {
            h.with_app_id("test1.near".parse::<AccountId>().unwrap());
        });

        let pk = sdk::host::crypto::ed25519::derive_public_key(path);
        println!("{pk:?}");
    }
}
