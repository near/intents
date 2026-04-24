use defuse_outlayer_sdk::host::crypto::ed25519;

fn main() {
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        println!("Read line: {line}");
    }

    ed25519::derive_public_key("some_path");
}
