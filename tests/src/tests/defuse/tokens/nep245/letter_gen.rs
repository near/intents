use std::iter::FusedIterator;

/// `LetterComb` is an iterator that yields all letter combinations
/// over the lowercase Latin alphabet, in order of increasing length.
#[must_use]
#[derive(Default)]
pub struct LetterCombinations {
    curr: Vec<usize>,
}

impl LetterCombinations {
    pub fn new() -> Self {
        Self { curr: vec![0] }
    }

    pub fn generate_combos(count: usize) -> Vec<String> {
        Self::new().take(count).collect()
    }
}

impl Iterator for LetterCombinations {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        // build string from curr
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::as_conversions)]
        let s = self
            .curr
            .iter()
            .map(|&i| (b'a' + i as u8) as char)
            .collect();
        // increment like a base-26 counter
        for i in (0..self.curr.len()).rev() {
            if self.curr[i] < 25 {
                self.curr[i] += 1;
                return Some(s);
            }
            self.curr[i] = 0;
        }
        // all rolled over → increase length
        self.curr = vec![0; self.curr.len() + 1];
        Some(s)
    }
}

impl FusedIterator for LetterCombinations {}

#[test]
fn test_first_letters() {
    let mut iter = LetterCombinations::new();
    let expected: Vec<String> = ('a'..='z').map(|c| c.to_string()).collect();
    let result: Vec<String> = iter.by_ref().take(26).collect();
    assert_eq!(result, expected);
}

#[test]
fn test_rollover_to_aa_and_ab() {
    let mut iter = LetterCombinations::new();
    // Skip 'a'..'z'
    for _ in 0..26 {
        iter.next();
    }
    assert_eq!(iter.next(), Some("aa".into()));
    assert_eq!(iter.next(), Some("ab".into()));
}

#[test]
fn test_sequence_continues_correctly() {
    let mut iter = LetterCombinations::new();
    // Collect first 28: "a".."z", "aa", "ab"
    let got: Vec<String> = iter.by_ref().take(28).collect();

    // build expected: 'a'..='z'
    let mut want: Vec<String> = ('a'..='z').map(|c| c.to_string()).collect();
    want.push("aa".into());
    want.push("ab".into());

    assert_eq!(got, want);
}

#[test]
fn test_third_length_start() {
    let mut iter = LetterCombinations::new();
    // Skip all 1- and 2-letter combos: 26 + 26*26 = 702
    for _ in 0..(26 + 26 * 26) {
        iter.next();
    }
    assert_eq!(iter.next(), Some("aaa".into()));
}
