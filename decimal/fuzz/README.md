## Setup
```sh
rustup install nightly
cargo install cargo-fuzz
```

## Fuzz
```sh
cargo +nightly fuzz run parse
cargo +nightly fuzz run roundtrip
```