use defuse_randomness::Rng;

/// Returns a new String where one character in `s` is replaced by a random lowercase ASCII letter.
pub fn tamper_string(rng: &mut impl Rng, s: &str) -> String {
    // change one character
    let mut chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    // Either add a character, or change a character
    if s.is_empty() || rng.random() {
        // Insert a character
        let idx: usize = rng.random_range(0..=len);
        #[allow(clippy::as_conversions)]
        let c = (b'a' + rng.random_range(0..26)) as char;
        chars.insert(idx, c);
    } else {
        let idx = rng.random_range(0..len);

        // keep sampling until we get a new char
        let new_c = loop {
            #[allow(clippy::as_conversions)]
            let c = (b'a' + rng.random_range(0..26)) as char;
            if c != chars[idx] {
                break c;
            }
        };
        chars[idx] = new_c;
    }

    chars.into_iter().collect()
}

/// Returns a new signature byte‐vector where exactly one bit of the original `sig`
/// has been flipped at a random position.
pub fn tamper_bytes(rng: &mut impl Rng, sig: &[u8], maybe_modify_size: bool) -> Vec<u8> {
    let mut tampered = sig.to_vec();
    if maybe_modify_size && (sig.is_empty() || rng.random()) {
        let len = sig.len();
        let idx: usize = rng.random_range(0..=len);
        #[allow(clippy::as_conversions)]
        let b = rng.random();
        tampered.insert(idx, b);
    } else {
        let total_bits = tampered.len() * 8;
        // pick a random bit index and flip it
        let bit_idx = rng.random_range(0..total_bits);
        let byte_idx = bit_idx / 8;
        let bit_in_byte = bit_idx % 8;
        tampered[byte_idx] ^= 1 << bit_in_byte;
    }
    tampered
}
