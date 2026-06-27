use std::fmt::Display;

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
