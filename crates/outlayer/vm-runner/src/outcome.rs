/// Details about the execution of a component
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub struct ExecutionDetails {
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::DisplayFromStr"))]
    pub fuel_consumed: u64,
}

/// Outcome of executing a component in the VM runtime
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutionOutcome {
    pub details: ExecutionDetails,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub error: Option<String>,
}

impl ExecutionOutcome {
    pub fn into_result(self) -> Result<(), String> {
        self.error.map_or(Ok(()), Err)
    }
}
