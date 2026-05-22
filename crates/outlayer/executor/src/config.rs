#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields, default))]
#[derive(Debug, Clone, Copy)]
pub struct ExecutorLimits {
    #[cfg_attr(feature = "serde", serde_as(as = "Clamp<1, { 64 * 1024 * 1024 }, usize>"))]
    pub stdin: usize,

    #[cfg_attr(feature = "serde", serde_as(as = "Clamp<1, { 64 * 1024 * 1024 }, usize>"))]
    pub stdout: usize,

    #[cfg_attr(feature = "serde", serde_as(as = "Clamp<1, { 1024 * 1024 }, usize>"))]
    pub stderr: usize,
}

impl Default for ExecutorLimits {
    fn default() -> Self {
        Self {
            stdin: 4 * 1024 * 1024,
            stdout: 4 * 1024 * 1024,
            stderr: 16 * 1024,
        }
    }
}

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields, default))]
#[derive(Debug, Clone, Copy)]
pub struct Config {
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "Clamp<{ 1024 * 1024 }, { 4 * 1024 * 1024 * 1024 }, usize>")
    )]
    pub memory_limit: usize,

    pub limits: ExecutorLimits,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            memory_limit: 100 * 1024 * 1024,
            limits: ExecutorLimits::default(),
        }
    }
}

#[cfg(feature = "serde")]
use defuse_outlayer_utils::Clamp;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executor_limits_clamps_below_min() {
        // stdin MIN is 1; passing 0 should be clamped to 1
        let json = r#"{"stdin": 0, "stdout": 1, "stderr": 1}"#;
        let limits: ExecutorLimits = serde_json::from_str(json).unwrap();
        assert_eq!(limits.stdin, 1);
    }
}
