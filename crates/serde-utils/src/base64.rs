use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

#[serde_as]
#[cfg_attr(
    feature = "schemars-v0_8",
    derive(::schemars::JsonSchema),
    schemars(transparent)
)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, From)]
#[serde(bound(serialize = "T: AsRef<[u8]>", deserialize = "T: TryFrom<Vec<u8>>"))]
/// Helper type to implement `#[derive(Serialize, Deserialize)]`,
/// as `#[near_bindgen]` doesn't support `#[serde(...)]` attributes on method arguments
#[repr(transparent)]
pub struct AsBase64<T>(#[serde_as(as = "Base64")] pub T);

impl<T> AsBase64<T> {
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}
