use std::fmt::Display;

/// Assert that collection `a` contains collection `b`.
/// Checks that all elements in `b` are present in `a`.
///
/// # Examples
/// ```ignore
/// assert_a_contains_b(a: all_logs, b: [expected_event1, expected_event2]);
/// ```
#[track_caller]
pub fn assert_a_contains_b(
    a: impl IntoIterator<Item: Display>,
    b: impl IntoIterator<Item: Display>,
) {
    let a: Vec<String> = a.into_iter().map(|v| v.to_string()).collect();
    let b: Vec<String> = b.into_iter().map(|v| v.to_string()).collect();

    for expected_event in &b {
        assert!(
            a.contains(expected_event),
            "\n\nExpected event not found in 'a':\n{expected_event}\n\nActual event logs in 'a':\n{a:#?}\n",
        );
    }
}

#[cfg(feature = "defuse")]
#[track_caller]
pub fn assert_eq_defuse_event_logs(
    left: impl IntoIterator<Item: Display>,
    right: impl IntoIterator<Item: Display>,
) {
    let standard = "\"standard\":\"dip4\"";

    let left: Vec<String> = left
        .into_iter()
        .map(|v| v.to_string())
        .filter(|s| s.contains(standard))
        .collect();

    let right: Vec<String> = right
        .into_iter()
        .map(|v| v.to_string())
        .filter(|s| s.contains(standard))
        .collect();

    assert_eq!(left, right);
}
