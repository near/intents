#[macro_export]
macro_rules! assert_eq_event_logs {
    ($left:expr, $right:expr) => {{
        let left: Vec<String> = $left.iter().map(ToString::to_string).collect();
        let right: Vec<String> = $right.iter().map(ToString::to_string).collect();
        assert_eq!(left, right);
    }};
}

#[cfg(feature = "defuse")]
#[macro_export]
macro_rules! assert_eq_defuse_event_logs {
    ($left:expr, $right:expr) => {{
        let standard = "\"standard\":\"dip4\"";

        let left: Vec<String> = $left
            .iter()
            .map(ToString::to_string)
            .filter(|s| s.contains(standard))
            .collect();

        let right: Vec<String> = $right
            .iter()
            .map(ToString::to_string)
            .filter(|s| s.contains(standard))
            .collect();

        assert_eq!(left, right);
    }};
}

/// Assert that collection `a` contains collection `b`.
/// Checks that all elements in `b` are present in `a`.
///
/// # Examples
/// ```ignore
/// assert_a_contains_b!(a: all_logs, b: [expected_event1, expected_event2]);
/// ```
#[macro_export]
macro_rules! assert_a_contains_b {
    (a: $a:expr, b: $b:expr) => {{
        let a: Vec<String> = $a.iter().map(ToString::to_string).collect();
        let b: Vec<String> = $b.iter().map(ToString::to_string).collect();

        for expected_event in &b {
            if !a.contains(expected_event) {
                panic!(
                    "\n\nExpected event not found in 'a':\n{}\n\nActual event logs in 'a':\n{:#?}\n",
                    expected_event,
                    a
                );
            }
        }
    }};
}
