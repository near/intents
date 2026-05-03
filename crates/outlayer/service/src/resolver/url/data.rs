use bytes::Bytes;
use data_url::{DataUrl, DataUrlError, forgiving_base64::InvalidBase64};

/// `data:[application/wasm][;base64],<data>`
///
/// See <https://developer.mozilla.org/en-US/docs/Web/URI/Reference/Schemes/data>
pub fn resolve(url: impl AsRef<str>) -> Result<Bytes, Error> {
    let url = DataUrl::process(url.as_ref())?;

    let (body, _fragment) = url.decode_to_vec()?;

    Ok(body.into())
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("base64: {0}")]
    Base64(#[from] InvalidBase64),

    #[error(transparent)]
    Parse(#[from] DataUrlError),
}
